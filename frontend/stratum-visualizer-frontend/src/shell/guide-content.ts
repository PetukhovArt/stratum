import type { Lang } from '../i18n'

export interface GuideView {
  key: string
  name: string
  text: string
}

export interface GuideCase {
  goal: string
  steps: readonly string[]
}

export interface GuideDoc {
  title: string
  intro: readonly string[]
  run: { title: string; lines: readonly string[] }
  views: { title: string; items: readonly GuideView[] }
  cases: { title: string; items: readonly GuideCase[] }
  controls: { title: string; items: readonly (readonly [string, string])[] }
  close: string
}

const RUN = [
  'stratum-lint visualize [path]',
  'stratum-lint visualize [path] --base develop',
  'stratum-lint visualize [path] --feature editor',
  'stratum-lint export --out DIR [--with-source]',
] as const

const en: GuideDoc = {
  title: 'How to use the visualizer',
  intro: [
    'The visualizer shows the imports of the project on top of the layers from stratum.config.jsonc. A red mark is an import that breaks a rule. The data is the same as the lint run.',
    'Pick a view with the tabs or the keys 1–9. The left panel lists what you can pick, the canvas draws it, the right panel explains the selection.',
  ],
  run: {
    title: 'Run',
    lines: [
      `${RUN[0]} — open the project`,
      `${RUN[1]} — also compare with a branch: adds the Merge request view`,
      `${RUN[2]} — open with the feature filter set`,
      `${RUN[3]} — write a static site you can share without the server`,
    ],
  },
  views: {
    title: 'Views',
    items: [
      { key: '1', name: 'Module focus', text: 'One module: who imports it, what it imports, one or two hops away. The right panel gives its layer, what it may import, and its findings.' },
      { key: '2', name: 'Matrix', text: 'Layers (or units) against layers. A row imports a column. A cell above the diagonal imports against the order; red cells break a rule. Dashed squares are cycles.' },
      { key: '3', name: 'Merge request', text: 'Only with --base. Imports the branch added or removed, rules they break, stage changes. Comment card gives a picture for a review comment.' },
      { key: '5', name: 'Module set', text: 'A group of modules, usually a cycle. Shows the shortest loop and the imports whose removal frees the most modules.' },
      { key: '6', name: 'Architecture', text: 'C4 levels: System, Containers, Components, Code. System shows the people who use the app (actors in the config) on top, the app and the other systems of the product in the middle and the external infrastructure below: solid arrows are measured contract calls or modules that reach a system directly (via), dotted arrows are relations declared in the config (uses). Containers draws the blocks and the links of imports between them; click a link to see its contract.' },
      { key: '9', name: 'Bench', text: 'Measures the views against their targets: speed, number of marks, questions answered in three actions.' },
    ],
  },
  cases: {
    title: 'Use cases',
    items: [
      {
        goal: 'I am new in a part of the code',
        steps: [
          'Header → Features: pick the feature, for example editor.',
          'Matrix (2): see which layers it touches and how they import each other.',
          'Architecture (6) → Containers: its blocks and the links between them. Double-click a block to open it.',
        ],
      },
      {
        goal: 'What breaks if I change this file?',
        steps: [
          'Press / and type the file name.',
          'Module focus (1): the left column imports the file. Switch to 2 hops for the wider circle.',
          'Add to set, then Module set (5) to look at the group.',
        ],
      },
      {
        goal: 'Review a merge request',
        steps: [
          'Run with --base <target branch>.',
          'Merge request (3): the right panel lists rule breaks and stage changes.',
          'Click a change to select its import on the canvas. Variant Comment card gives a picture for the review.',
        ],
      },
      {
        goal: 'Who uses the contract of a feature?',
        steps: [
          'Architecture (6) → Containers: click the link between two blocks.',
          'The right panel lists the names that cross it and the modules that use them.',
          'Click a name: the Code panel opens at its declaration. Esc closes it.',
        ],
      },
      {
        goal: 'Break an import cycle',
        steps: [
          'Matrix (2): a dashed square is a cycle.',
          'Module set (5): pick a cycle in the left panel.',
          'Cut candidates list the imports to remove first, with the number of modules each one frees.',
        ],
      },
      {
        goal: 'Which calls go to the backend or the desktop host?',
        steps: [
          'Architecture (6), level System.',
          'Click the line to a system: the contracts it uses, the callers, and their features.',
          'Click a caller to open its code.',
        ],
      },
      {
        goal: 'Which modules touch the disk or another system without contracts?',
        steps: [
          'Architecture (6), level System.',
          'Click the arrow labelled "via N modules": the details list the modules matched by the via globs of that system.',
          'Click a module to open its code, double-click it to open its Module focus.',
        ],
      },
      {
        goal: 'How far are we from the declared architecture?',
        steps: [
          'Matrix (2): the share above the diagonal and the number of rule breaks.',
          'Variant Units for a closer look inside the layers.',
        ],
      },
    ],
  },
  controls: {
    title: 'Controls',
    items: [
      ['1–9', 'switch view'],
      ['v', 'next variant of the view'],
      ['f', 'fit the canvas'],
      ['/ · Ctrl K', 'find a module or a command'],
      ['Esc', 'close the Code panel, then go back'],
      ['Ctrl [ · Ctrl ]', 'hide or show the left or right panel (also the buttons in the status bar corners)'],
      ['?', 'open this guide'],
      ['h · t (Module focus)', 'one or two hops · hide or show the import rules strip'],
      ['drag the background', 'pan'],
      ['wheel', 'zoom'],
      ['double-click a block (Architecture → Containers)', 'open it'],
      ['click a link or a hub chip (Architecture → Containers)', 'its contract'],
      ['double-click a module (Architecture → Components)', 'open its code'],
      ['× on the legend', 'fold the legend'],
    ],
  },
  close: 'Close',
}

const ru: GuideDoc = {
  title: 'Как пользоваться визуализатором',
  intro: [
    'Визуализатор показывает импорты проекта поверх слоёв из stratum.config.jsonc. Красная метка — импорт, который нарушает правило. Данные те же, что у прогона линтера.',
    'Вид выбирается вкладкой или клавишами 1–9. Слева — список, из которого выбирают, в центре — схема, справа — объяснение выбранного.',
  ],
  run: {
    title: 'Запуск',
    lines: [
      `${RUN[0]} — открыть проект`,
      `${RUN[1]} — сравнить с веткой: появится вид Merge request`,
      `${RUN[2]} — открыть с фильтром по фиче`,
      `${RUN[3]} — собрать статический сайт, его можно отдать без сервера`,
    ],
  },
  views: {
    title: 'Виды',
    items: [
      { key: '1', name: 'Module focus', text: 'Один модуль: кто его импортирует и что импортирует он, на один-два шага. Справа — его слой, что ему можно импортировать и его нарушения.' },
      { key: '2', name: 'Matrix', text: 'Слои (или юниты) против слоёв. Строка импортирует столбец. Клетка выше диагонали — импорт против порядка, красная — нарушение. Пунктирный квадрат — цикл.' },
      { key: '3', name: 'Merge request', text: 'Только с --base. Импорты, которые ветка добавила или убрала, их нарушения и смены стадий. Comment card даёт картинку для комментария в ревью.' },
      { key: '5', name: 'Module set', text: 'Группа модулей, обычно цикл. Видно самую короткую петлю и импорты, удаление которых освобождает больше всего модулей.' },
      { key: '6', name: 'Architecture', text: 'Уровни C4: System, Containers, Components, Code. На System сверху люди, которые работают с приложением (actors в конфиге), в середине приложение и другие системы продукта, внизу внешняя инфраструктура: сплошные стрелки — измеренные вызовы контрактов или модули, которые работают с системой напрямую (via), пунктирные — связи, объявленные в конфиге (uses). На Containers — блоки и связи импортов между ними; клик по связи показывает её контракт.' },
      { key: '9', name: 'Bench', text: 'Меряет виды по целям: скорость, число меток, ответ на вопрос за три действия.' },
    ],
  },
  cases: {
    title: 'Сценарии',
    items: [
      {
        goal: 'Я впервые в этой части кода',
        steps: [
          'В шапке Features: выберите фичу, например editor.',
          'Matrix (2): какие слои она задевает и как они импортируют друг друга.',
          'Architecture (6) → Containers: её блоки и связи между ними. Двойной клик по блоку откроет его.',
        ],
      },
      {
        goal: 'Что сломается, если я поменяю этот файл?',
        steps: [
          'Нажмите / и наберите имя файла.',
          'Module focus (1): левая колонка импортирует файл. Переключите на 2 шага, чтобы увидеть круг шире.',
          'Add to set, затем Module set (5) — посмотреть группу целиком.',
        ],
      },
      {
        goal: 'Ревью merge request',
        steps: [
          'Запустите с --base <целевая ветка>.',
          'Merge request (3): справа список нарушений и смен стадий.',
          'Клик по изменению выделит импорт на схеме. Вариант Comment card — картинка для ревью.',
        ],
      },
      {
        goal: 'Кто пользуется контрактом фичи?',
        steps: [
          'Architecture (6) → Containers: клик по связи между двумя блоками.',
          'Справа — имена, которые через него идут, и модули, которые их берут.',
          'Клик по имени откроет панель Code на объявлении. Esc её закроет.',
        ],
      },
      {
        goal: 'Разорвать цикл импортов',
        steps: [
          'Matrix (2): пунктирный квадрат — цикл.',
          'Module set (5): выберите цикл в левой панели.',
          'Cut candidates — какие импорты убрать первыми и сколько модулей это освободит.',
        ],
      },
      {
        goal: 'Какие вызовы идут в backend или в десктоп-хост?',
        steps: [
          'Architecture (6), уровень System.',
          'Клик по линии к системе: её контракты, кто их вызывает и из каких фич.',
          'Клик по вызову откроет код.',
        ],
      },
      {
        goal: 'Какие модули работают с диском или другой системой без контрактов?',
        steps: [
          'Architecture (6), уровень System.',
          'Клик по стрелке с подписью «через N модулей»: в деталях модули, подходящие под глобы via этой системы.',
          'Клик по модулю откроет код, двойной клик — его Module focus.',
        ],
      },
      {
        goal: 'Насколько мы отъехали от заявленной архитектуры?',
        steps: [
          'Matrix (2): доля выше диагонали и число нарушений.',
          'Вариант Units — подробнее внутри слоёв.',
        ],
      },
    ],
  },
  controls: {
    title: 'Управление',
    items: [
      ['1–9', 'сменить вид'],
      ['v', 'следующий вариант вида'],
      ['f', 'вписать схему'],
      ['/ · Ctrl K', 'найти модуль или команду'],
      ['Esc', 'закрыть панель Code, потом шаг назад'],
      ['Ctrl [ · Ctrl ]', 'скрыть или показать левую или правую панель (или кнопки в углах строки статуса)'],
      ['?', 'открыть эту справку'],
      ['h · t (Module focus)', 'один или два шага · скрыть или показать полосу правил импорта'],
      ['тянуть фон', 'сдвиг'],
      ['колесо', 'масштаб'],
      ['двойной клик по блоку (Architecture → Containers)', 'открыть его'],
      ['клик по связи или плашке хаба (Architecture → Containers)', 'её контракт'],
      ['двойной клик по модулю (Architecture → Components)', 'открыть код'],
      ['× на легенде', 'свернуть легенду'],
    ],
  },
  close: 'Закрыть',
}

export const GUIDE: Record<Lang, GuideDoc> = { en, ru }
