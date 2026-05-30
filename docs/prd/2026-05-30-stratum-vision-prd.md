# PRD — Stratum: архитектурные guardrails для TS/Vue (видение + roadmap)

> **Статус:** draft v2 · 2026-05-30 (переписан после грилл-сессии)
> **Формат:** видение → концептуальный фундамент → каталог правил → TODO по фазам
> **Решения зафиксированы** в грилл-таблице (Приложение B). v1 содержал слой state-management (DI-скоупы) — **выпилен**, фокус на чистых архитектурных концептах.

## 0. Что изменилось против v1

- **Убран весь слой state-management** (DI-скоупы, safe-context/inject-ban, Pinia, composer-owns-scope). Это runtime-стейт, не архитектурные границы.
- **Фундамент переопределён через концепты:** DAG · deep modules · fractal · modules evolution · layers boundaries · stable dependencies (instability) · private-to-siblings. Bounded contexts — отложены.
- **Каталог правил — 3 яруса** с явным маппингом из Steiger.
- **Коллизия `Stage` разрешена:** `Stage` = эволюция (headline-концепт), старый purity → `Purity`.
- **Визуализатор = lens-плагины**, привязанные к подключённым правилам конфига.
- **SVG-рендер удаляем** — WebGL единственный.

---

## 1. Видение и позиционирование

### 1.1. One-liner

> **AI пишет код в 5–7× быстрее, чем команда успевает его читать, и не знает твою архитектуру. Stratum — детерминированный guardrail и живая карта, которые держат DAG, границы слоёв, глубину модулей и стадии эволюции TS/Vue-кодовой базы целыми, пока агенты и люди двигаются быстро.**

### 1.2. Почему сейчас

Проверяемые данные (якорим публичные заявления только на них):

- **GitClear (211M строк, 2020–2024):** дублирование кода ×8–10; доля «перемещённого» кода (сигнатура рефакторинга/переиспользования) 25% → <10%.
- **Google DORA 2024:** рост adoption AI → **−7.2% стабильности доставки**.
- **Comprehension-исследование (2026):** с AI разработчики поняли код на **17% хуже**; AI-PR ~×12 дороже в ревью.
- Дискурс 2026 («guardrails for agentic coding», «cross-layer edit problem») **прямо называет ArchUnit-style проверки зависимостей** необходимым guardrail. На фронтенде такого нет.

### 1.3. Конкурентная карта — где дыра

Рынок делится надвое, пересечение пусто:

| Кластер | Инструменты | Чего НЕ могут |
|---|---|---|
| Быстрые бинарные boundary-линтеры | Steiger, dependency-cruiser, eslint-plugin-boundaries, Nx, oxlint, Sheriff | Только «можно ли A→B». Нет *сколько / насколько глубоко / насколько зрело / куда тренд*. Нет интерактивной карты |
| Богатые метрики / поведение | ArchUnitTS, NDepend (.NET), CodeScene | Не enforce-ят инварианты вживую, либо не TS/Vue-native, либо живут в test-runner/сервере. Нет временно́й оси |

**Ни у кого нет временно́й оси (evolution/promotion) и ни у кого FSD-aware enforcement не соединён с интерактивной визуализацией.** Скорость — table stakes (oxlint уже делает OXC+граф ~7с/126k файлов), а не дифференциатор. Моат — *стек, который никто не собрал*: Rust/Salsa-инкрементальность → LSP live → WebGL-карта → словарь правил с временно́й осью → MCP для AI.

### 1.4. Headline-фичи (нельзя сформулировать у конкурентов)

1. **Modules evolution + promotion** — временна́я ось в правилах.
2. **Deep modules через deletion-test** (pass-through detection) — операционализация Ousterhout, концепт без тулинга.
3. **Fractal-рекурсия** — одни правила на каждом уровне вложенности.
4. **Private-to-siblings** — структурный enforcement антипаттерна «shared = свалка».
5. **Всё это live в редакторе, анимировано на WebGL-карте, и читается AI-агентом через MCP до правки.**

---

## 2. Концептуальный фундамент

Каждый концепт — с операционным определением (как Stratum его *детектит*) и enforcement-формой.

### 2.1. DAG
Граф импортов модулей — направленный ациклический. **Детект:** циклы в `deps` (petgraph). **Enforce:** `no-circular-deps`. Уже есть.

### 2.2. Layers boundaries
Импорты уважают объявленный в конфиге `depends_on`; горизонталь (между слайсами одного слоя) запрещена по умолчанию. **Детект:** слой источника vs разрешённые слои; слайс источника vs слайс цели в одном слое. **Enforce:** `no-cross-layer-import` (есть) + `no-cross-slice-import` (новое).

### 2.3. Deep modules
Глубокий = много поведения за узким интерфейсом. Shallow = интерфейс почти так же сложен, как реализация (pass-through). **Главный сигнал — deletion-test как граф-операция:** модуль, чьи публичные экспорты в основном ре-экспортят его собственные зависимости, не добавляя своих узлов = pass-through = shallow. **Вторичный** — ratio `публичные экспорты / внутренние реализационные модули`. Граф не видит инварианты/порядок/ошибки интерфейса — это прокси, и так пишем в сообщении. **Enforce:** `deep-module` (info/warning). **Механический фундамент глубины** — `enforce-public-api` (узкий интерфейс через barrel).

### 2.4. Fractal
Один шаблон (стадия, Miller, deep-module) повторяется на каждом уровне иерархии. **Детект:** правила применяются рекурсивно к каждому контейнеру, стадия вычисляется per-уровень (`pages/admin` stage-4 → `permissions/` stage-4 → `video-server-table/` stage-3 — каждый отдельно). Это enforcement-свойство движка, не картинка.

### 2.5. Modules evolution
Стадия модуля: `File → Flat → Grouped → Composed` (1→4). Триггеры — Миллер и число разнотипных детей, не LOC. **Детект — автоматический** по структуре папки (число файлов, разнотипность, наличие `model/api/ui`, наличие подфич). Ручная аннотация `@stratum-stage` — только override. **Enforce:** `evolution-stage` (вычисление + бейдж), `stage-mismatch` (смешение сегментов и подфич на одном уровне; сегменты ожидаются только со stage-3+). Промоут наружу по числу потребителей — `promotion-pressure` (есть).

### 2.6. Stable Dependencies (instability)
Зависимости направлены от нестабильного к стабильному. Метрика `I = Ce / (Ca + Ce)` (исходящие / (входящие + исходящие)). **Детект:** считается на графе дёшево. **Enforce:** `instability` — advisory-метрика + флаг SDP-нарушения (модуль зависит от более нестабильного, чем он сам). Даёт «направление зрелости» поверх «без циклов». Cohesion/LCOM — отложено (шумно, дорого).

### 2.7. Private-to-siblings (инкапсуляция)
Приватная папка видна только прямым братьям + родителю. Обобщение DMFA-`_shared/` без привязки к имени. **Детект:** импорт такой папки запрещён, если путь прыгает через чужого родителя. Имена конфигурируемы (дефолт `_shared`, `_internal`). **Enforce:** `private-to-siblings` (переработка существующего `visibility-scope`). Усиливает фрактальную инкапсуляцию на каждом уровне.

### 2.8. Bounded contexts — отложено
Вертикальный домен через слои (DDD-граница). Признан **не нужным сейчас** — не первоклассная сущность движка. Если вернётся: авто-группировка по именам слайсов между слоями + явный маппинг. Зафиксировано как deferred, чтобы будущий explorer не предлагал заново.

---

## 3. Модель данных: что добавляем

| Что | Сейчас | Нужно |
|---|---|---|
| **Символьный анализ** | `deep-module` использует рёбра как прокси экспортов; named imports не парсятся | **P0:** парсить named imports/exports в `stratum-parser-ts` (OXC даёт AST). `Module.exports: Vec<ExportSymbol>`, `Edge.imported_names`, `Edge.import_depth` (глубина пути относительно public API). Блокер для `enforce-public-api`, честного `deep-module`, `instability` |
| **Slice** | контейнеры есть, slice не выделен | **P0:** Slice = контейнер 1-го уровня под layer-root. Пререкизит `no-cross-slice-import`, `insignificant-slice` |
| **EvolutionStage** | нет | Новая ось на контейнере: `File\|Flat\|Grouped\|Composed`, авто-детект |
| **Purity (ex-Stage)** | `Stage(u8)` purity, rule `stage-purity` | Переименовать `Stage`→`Purity`, slug `stage-purity`→`purity-order` + алиас на minor; понизить до опционального advisory, не удалять |
| **Config-валидация** | — | `layer.path` указывает на существующую папку (config-lint вместо неприменимого `typo-in-layer-name`) |

---

## 4. Каталог правил — 3 яруса

Пресеты подключаются через `extends` в конфиге: `vanilla-fsd`, `fsd-canonical`, `dmfa`. Severity у каждого правила — дефолт (hard/advisory), полностью переопределяется в конфиге вплоть до `off`.

### Ярус 1 — универсальный движок (любой TS/Vue с layers в конфиге)

| Правило | Концепт | Дефолт severity | Статус |
|---|---|---|---|
| `no-cross-layer-import` | DAG / layers | error | есть |
| `no-circular-deps` | DAG | error | есть, починить отчёт на больших SCC |
| `no-cross-slice-import` | layers (горизонталь) | error | новое |
| `enforce-public-api` | deep modules (узкий интерфейс) | error | новое (символьный) |
| `require-public-api` | deep modules | error | новое (slice без index.ts) |
| `private-to-siblings` | инкапсуляция | error | переработка `visibility-scope` |
| `deep-module` | deep modules | warning | переработка (pass-through + ratio) |
| `instability` | stable dependencies | info | новое |
| `miller-limit` | fractal / cognitive load | warning | переработка (рекурсия, homo/hetero, база 6) |
| `promotion-pressure` | evolution | info | есть, калибровка |
| `insignificant-slice` | evolution (fan-in≈0) | info | новое (adapt Steiger) |
| `cross-entity-pattern` | fan-out | warning | есть, калибровка |
| `config-paths-exist` | config-валидация | error | новое |

### Ярус 2 — FSD-пресет (канон-структура FSD)

| Правило | Из Steiger | Дефолт |
|---|---|---|
| `no-segments-on-sliced-layers` | =same | warning |
| `no-layer-public-api` | =same | warning |
| `no-ui-in-app` | =same | warning |
| `ambiguous-slice-names` | =same | warning |
| `no-reserved-folder-names` | =same | info |
| `segments-by-purpose` | =same | info |

### Ярус 3 — DMFA-пресет

| Правило | Концепт | Дефолт |
|---|---|---|
| `evolution-stage` | modules evolution (детект+бейдж) | info |
| `stage-mismatch` | fractal (заменяет `no-segmentless-slices`) | warning |

### Опционально (вне пресетов)

- `purity-order` (ex-`stage-purity`) — advisory, доступно, но не в дефолтных пресетах. Удалить позже, если не востребовано.

### Маппинг Steiger → Stratum (полный)

| Steiger | Stratum | Судьба |
|---|---|---|
| forbidden-imports / no-cross-imports / no-higher-level-imports | `no-cross-layer-import` + `no-cross-slice-import` | покрыто |
| public-api | `require-public-api` | адаптировано |
| no-public-api-sidestep | `enforce-public-api` | адаптировано |
| excessive-slicing + shared-lib-grouping | рекурсивный `miller-limit` | покрыто |
| insignificant-slice | `insignificant-slice` | адаптировано |
| no-segments-on-sliced-layers / no-layer-public-api / no-ui-in-app / segments-by-purpose | FSD-пресет (те же) | ярус 2 |
| ambiguous-slice-names / no-reserved-folder-names | арх-значимый naming | ярус 2 |
| inconsistent/repetitive-naming, typo-in-layer-name | — | вне скоупа (Biome); typo → `config-paths-exist` |
| no-segmentless-slices | `stage-mismatch` | заменено (конфликт с evolution) |
| no-processes | — | неприменимо (слои в конфиге) |
| import-locality | — | отложено (стиль) |

---

## 5. Визуализатор — lens-плагины

**Принцип: lens-оверлеи поверх одного графа, привязанные к подключённым правилам конфига.** Отдельного списка «что рисовать» нет — включил правило, появилась его линза.

- Базовый граф: модули + рёбра + папочная иерархия (рекурсивная, уже есть).
- Lens (один активен за раз, тумблеры):
  - **Stages** — бейдж/цвет стадии эволюции на контейнере.
  - **Depth** — deep = насыщенный, shallow/pass-through = блёклый/штриховка.
  - **Instability** — градиент по `I`.
  - **Violations** — подсветка нарушающих рёбер (cross-slice, deep-import, private-to-siblings).
  - **Promotion** — пульсация узлов, которые пора выносить наружу.
- **SVG-рендер удаляется.** Перед удалением закрыть паритет: edge-hover тултип, dashed cycle edges. Потом — drop `renderer=svg`, флаг рендерера, мёртвый код.

---

## 6. Severity-философия

Полностью настраивается в конфиге (механика `RuleConfig.severity` есть). Дефолты — лишь стартовая точка:

- **Hard error по дефолту (бинарно, объективно):** DAG/циклы, layer boundaries, cross-slice, public-api, private-to-siblings, config-paths.
- **Advisory по дефолту (warning/info, калибруется):** deep-module, Miller, instability, stage-mismatch, promotion, cross-entity, insignificant-slice.
- Метрику никогда не делаем error по дефолту (риск «кивают, но не gate-ят») — команда поднимает осознанно.

---

## 7. Roadmap по фазам (TODO)

Приоритет: **P0** блокер · **P1** ядро · **P2** усиление.

### Фаза v0.2 — «Символьный фундамент + ярус 1» (детально)

Цель: символьный анализ + slice + ключевые универсальные правила. Метрика: на web-client включены ≥6 правил яруса 1, выборочный аудит — ≥80% срабатываний actionable.

**Фундамент**
- [ ] **P0** Символьный анализ named imports/exports (OXC) → `Module.exports`, `Edge.imported_names`, `Edge.import_depth`. → verify: fixture различает `import {x} from '../sib/model/store'` (deep) и `from '../sib'` (public).
- [ ] **P0** Slice как сущность графа (контейнер 1-го уровня под layer). → verify: на web-client выделяются 14 features-слайсов, 6 entities.
- [ ] **P0** Коллизия `Stage`: `Stage`→`Purity`, `stage-purity`→`purity-order` + алиас. → verify: `cargo test --workspace` зелёный; старый slug даёт deprecation-warning, не ошибку.

**Правила яруса 1**
- [ ] **P1** `no-cross-slice-import` — feature↛feature. Опция `allow: []`. → verify: ловит реальные cross-slice на web-client, список не шумный.
- [ ] **P1** `enforce-public-api` — deep-import ban по `import_depth`. → verify: `../archive-export/model/store` падает, `../archive-export` ок.
- [ ] **P1** `require-public-api` — slice без `index.ts`. → verify: слайс без barrel флагается.
- [ ] **P1** `private-to-siblings` — переработка `visibility-scope`, path-jump detection, конфиг имён. → verify: `permissions/→admin/_shared/` ок, `infrastructure/→permissions/_shared/` падает.
- [ ] **P1** `deep-module` — pass-through detection (основной) + ratio (вторичный), info/warning, прокси-природа в сообщении. → verify: 1 экспорт над 10 файлами = deep; ре-экспорт-барель = shallow.
- [ ] **P2** `instability` — `I = Ce/(Ca+Ce)` + SDP-нарушение, info. → verify: метрика на узле совпадает с ручным подсчётом на fixture.
- [ ] **P2** `miller-limit` — рекурсия по уровням, homo/hetero, `maxHeterogeneous: 6` / `maxHomogeneous: 20`. → verify: `permissions/` (7 однотипных таблиц) НЕ флагается; 7 разнотипных — флагается.
- [ ] **P2** `insignificant-slice` — fan-in≈0. `config-paths-exist` — валидация `layer.path`.

**Визуализатор / инфра**
- [ ] **P1** Lens «Violations» для новых правил.
- [ ] **P1** Калибровка порогов на web-client (сейчас 202 срабатывания — отделить сигнал).
- [ ] **P2** Починить `no-circular-deps` отчёт на больших SCC (953-узловой цикл). CI-job `web-client-integration` через `WEB_CLIENT_PATH`.

### Фаза v0.3 — «Evolution + fractal + пресеты» (мазками)
- [ ] `EvolutionStage` ось + авто-детектор по структуре папки.
- [ ] `stage-mismatch` (сегменты+подфичи на одном уровне; сегменты только со stage-3+).
- [ ] Рекурсивное применение всех container-правил по уровням (фрактал как enforcement).
- [ ] Lens «Stages» / «Depth» / «Instability» / «Promotion».
- [ ] Пресеты `vanilla-fsd` / `fsd-canonical` / `dmfa` через `extends`. Ярус 2 (FSD-канон правила).
- [ ] **Drop SVG:** закрыть паритет (edge-hover, dashed cycle) → удалить SVG-рендер, флаг, мёртвый код.

### Фаза v0.4 — «AI-guardrails / MCP» (мазками) — self-selling
- [ ] **MCP-сервер** (тонкая обёртка поверх движка): `stratum_check_edit(file, proposed_import)` → вердикт правила; `stratum_layer_map()`; `stratum_where_should_this_go(desc)`.
- [ ] «Pre-edit guard»: агент читает правила до правки (атакует cross-layer-edit problem).
- [ ] Drift-режим: ratchet-бюджеты («≤N циклов, число не растёт») как CI-gate — временна́я ось.
- [ ] Компактный текстовый «architecture map» для LLM-контекста (против comprehension debt).

### Фаза v1.0 — «Продукт» (мазками)
- [ ] `stratum init` с выбором пресета. Документация: «ArchUnit для фронтенда» лендинг, рецепты правил, миграционные тактики (bridge-adapter / parallel-flag).
- [ ] Релиз: GPG-подпись, `cargo publish`, 10K-file cold-lint бенч, IDE-скриншоты.

---

## 8. Критерии успеха

| Уровень | Метрика |
|---|---|
| Технический | ≥8 правил на web-client; ≥80% срабатываний actionable; cold-lint 10k файлов < 5с |
| Продуктовый | Демо «live в редакторе + анимация на карте» даёт «I've never seen a tool say that» на ≥3 уникальных правилах (deep/evolution/instability) |
| Нарративный | Лендинг ведёт с AI-guardrails; ≥3 проверяемых дата-поинта (GitClear/DORA/comprehension) |
| Адопшн | Land-and-expand: коммодити-правила (cycles/layers/orphans) = подмножество, переход «ничего не теряешь» |

## 9. Риски

- **R1 · Стадии не поддерживают руками.** Митигация: авто-детект по структуре, аннотация — только override.
- **R2 · Метрики, на которые кивают, но не gate-ят.** Митигация: каждое метрик-правило даёт actionable suggestion, не голое число; не error по дефолту.
- **R3 · SonarQube «architecture as code» (2025)** — ближайший декларативный конкурент. Митигация: live-LSP + интерактивная карта + временна́я ось.
- **R4 · oxlint владеет нарративом скорости (тот же OXC).** Митигация: конкурируем словарём правил + визуализацией + MCP, не скоростью.
- **R5 · Символьный анализ — большой кусок.** Митигация: спайк на named imports первой неделей как P0-gate.
- **R6 · Удаление SVG до закрытия паритета теряет фичи.** Митигация: drop только после edge-hover + dashed cycle edges.

## 10. Открытые вопросы
1. MCP — отдельный крейт `stratum-mcp` или режим `stratum-lsp`?
2. Когда удалять `purity-order` совсем (или оставить как нишевый advisory)?
3. Bounded contexts — триггер на расконсервацию (если появится мультидоменный кейс)?

---

## Приложение A — источники (для лендинга/README)
FSD-тулинг: Steiger (issue #113 — нет кастомных слоёв), Sheriff, @feature-sliced/eslint-config (заброшен). Общие: dependency-cruiser (~6.7k★), eslint-plugin-boundaries, Nx, Madge, ts-arch/ArchUnitTS, Knip (~11.3k★), oxlint (~7с/126k файлов), Biome (#6245), ArchUnit. Метрики: NDepend (CQLinq/DSM), CodeScene, SonarQube architecture-as-code (2025.3). AI: GitClear (×8–10), DORA 2024 (−7.2%), comprehension (−17%), «guardrails for agentic coding» (2026-02), «cross-layer edit problem» (2026-03).

## Приложение B — грилл-таблица решений (2026-05-30)

| # | Решение | Выбор |
|---|---|---|
| 1 | Bounded contexts | отложено |
| 2 | Evolution: авто-детект по структуре, аннотация = override | user |
| 3 | Fractal: правила рекурсивно на каждом уровне | user |
| 4 | Miller: homo/hetero, база 6 / 20, конфигурируемо | user |
| 5 | Deep module: pass-through (осн.) + ratio (втор.), info/warning | user |
| 6 | `_shared` → обобщённый private-to-siblings | user |
| 7 | + Stable Dependencies (instability); cohesion отложен | user |
| 8 | Severity полностью конфигурируема; hard/advisory = дефолты | user |
| 9 | Визуализатор: lens-плагины, привязанные к правилам конфига | user |
| 10 | Каталог 3 яруса; segmentless → stage-mismatch | user |
| 11 | `Stage` = evolution; старый → `Purity` | user |
| 12 | Только арх-значимый naming + config-валидация путей | user |
| 13 | MCP — deliverable v0.4; AI-guardrails лид-нарратив | proposed |
| 14 | Символьный анализ named imports — P0 пререкизит | proposed |
| 15 | Slice = контейнер 1-го уровня под слоем | proposed |
| 16 | `enforce-public-api` + `no-cross-slice` в ярус 1 | proposed |
| 17 | `insignificant-slice` ярус 1 | proposed |
| 18 | DAG/layers/circular/promotion/cross-entity сохраняем | proposed |
| 19 | Пресеты через `extends`: vanilla-fsd / fsd-canonical / dmfa | proposed |
| 20 | SVG-рендер удаляем (после закрытия паритета) | user |
| 21 | Roadmap: v0.2 детально → v0.3 → v0.4 → v1.0 | proposed |
