# Техническое задание: UX-развитие Papyrus TUI

## 1. Цель

Расширить Papyrus как keyboard-first терминальный менеджер научной библиотеки.

Основные цели:

- ускорить навигацию по большой библиотеке;
- убрать зависимость пользовательского состояния от индексов списков;
- добавить полноценную работу со статьями и коллекциями;
- улучшить полнотекстовый поиск;
- добавить Markdown-заметки для курсов, проектов и отдельных документов;
- добавить fuzzy-picker как общий UI-компонент;
- сохранить терминальный характер приложения и минимизировать количество модальных действий;
- не усложнять `papyrus-core` там, где функцию можно реализовать на уровне TUI.

Существующие возможности должны сохраняться:

- SQLite через `rusqlite`;
- Tantivy для полнотекстового поиска;
- иерархические коллекции;
- many-to-many связь Paper ↔ Collection;
- теги;
- редактируемый TOC;
- открытие PDF на конкретной странице;
- экспорт ZIP;
- внешний PDF viewer;
- Ratatui + Crossterm.

---

# 2. Базовые архитектурные принципы

## 2.1. UUID как identity, индекс как UI-cache

Текущие `selected_collection`, `selected_paper`, `selected_toc` не должны быть единственным источником истины.

Ввести устойчивое состояние:

```rust
struct SelectionState {
    collection: CollectionKey,
    paper_id: Option<Uuid>,
    toc_id: Option<Uuid>,
}
```

Индексы:

```rust
selected_collection: usize,
selected_paper: usize,
selected_toc: usize,
```

сохраняются как вычисляемый UI-cache для Ratatui.

После:

- сортировки;
- collapse/expand;
- reload из SQLite;
- добавления/удаления коллекции;
- поиска;
- переключения виртуальных коллекций;

выделение должно восстанавливаться по UUID, а индекс пересчитываться.

Запрещено полагаться на прежний индекс после изменения порядка элементов.

---

## 2.2. CollectionKey

Отказаться от модели:

```rust
Option<Uuid>
```

где `None` означает `All Papers`.

Ввести:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CollectionKey {
    All,
    Real(Uuid),
    RecentlyAdded,
    Unfiled,
    Untagged,
}
```

В дальнейшем enum должен позволять добавлять другие виртуальные коллекции без fake UUID и вложенных `Option`.

---

## 2.3. Память позиции

Добавить:

```rust
last_paper_by_collection: HashMap<CollectionKey, Uuid>
last_toc_by_paper: HashMap<Uuid, Uuid>
```

Поведение:

- при уходе из коллекции сохраняется текущая статья;
- при возврате восстанавливается статья;
- при уходе со статьи сохраняется текущий TOC-узел;
- при возврате к статье восстанавливается TOC-позиция;
- если объект удалён, выбирается ближайший допустимый элемент.

---

# 3. Режимы интерфейса

## 3.1. Обычный трёхпанельный режим

Сохранить базовую структуру:

```text
┌ Collections ┐ ┌ Papers ────────────────┐ ┌ Context ───────────┐
│             │ │                        │ │ Details / TOC      │
│             │ │                        │ │ или Notes          │
└─────────────┘ └────────────────────────┘ └────────────────────┘
```

Базовые пропорции:

```text
Collections 25%
Papers      45%
Context     30%
```

Правая панель должна поддерживать режимы:

```rust
enum ContextView {
    Details,
    Toc,
    Notes,
}
```

---

## 3.2. Однопанельный режим

Добавить возможность по хоткею развернуть текущую рабочую область на весь терминал.

Поддерживаемые fullscreen-панели:

```text
Collections only
Papers only
Notes only
```

Дополнительно допускается существующий fullscreen TOC.

Предлагаемый хоткей:

```text
Ctrl-w
```

Поведение:

```text
Ctrl-w
```

в обычном режиме:

- если активна Collections → показать только Collections;
- если активна Papers → показать только Papers;
- если активна Notes → показать только Notes;
- если активна Details/TOC → допускается оставить существующий fullscreen TOC через `t`.

Повторный:

```text
Ctrl-w
```

возвращает предыдущий multi-panel layout.

Дополнительно:

```text
Esc
```

в single-panel режиме возвращает multi-panel layout, если не открыт modal.

Ввести:

```rust
enum LayoutMode {
    MultiPanel,
    SinglePanel,
}
```

Текущая активная панель определяет содержимое `SinglePanel`.

При переключении layout состояние курсора и scroll offset не сбрасывается.

---

## 3.3. Быстрый переход к конкретной панели

Сохранить:

```text
h / l
```

как переход между основными панелями.

Рекомендуемая логика:

```text
Collections ↔ Papers ↔ Context
```

Context использует выбранный `ContextView`.

Дополнительно:

```text
n    Notes
t    TOC
i    Details
```

при нахождении в Context.

Если пользователь нажимает `n` из Papers, фокус может сразу перейти в Context с `ContextView::Notes`.

---

# 4. Навигационный слой

## 4.1. Базовые motions

Поддержать одинаково в:

- Collections;
- Papers;
- TOC;
- Notes list/navigation;
- picker;
- fullscreen вариантах.

Команды:

```text
j / ↓        вниз
k / ↑        вверх

gg           первый элемент
G            последний элемент
12G          перейти к элементу 12

Ctrl-d       вниз на половину видимой страницы
Ctrl-u       вверх на половину видимой страницы
```

---

## 4.2. Числовые префиксы

Добавить:

```rust
pending_count: Option<usize>
```

Примеры:

```text
5j
10k
20G
```

Count применяется к следующему motion.

После выполнения команды count сбрасывается.

При нажатии `Esc` count также сбрасывается.

---

## 4.3. Общая motion abstraction

Не реализовывать каждую комбинацию отдельным кодом в `dispatch.rs`.

Ввести:

```rust
enum Motion {
    Relative(isize),
    First,
    Last,
    Absolute(usize),
    HalfPageDown,
    HalfPageUp,
}
```

И общий обработчик:

```rust
fn apply_motion(&mut self, motion: Motion)
```

Специфическая логика панели должна быть инкапсулирована отдельно.

---

# 5. Навигация между панелями

Добавить:

```text
h        предыдущая панель
l        следующая панель
```

Сохранить:

```text
Tab
BackTab
```

как альтернативу.

В Collections:

```text
Enter
```

должен:

1. выбрать коллекцию;
2. перейти в Papers.

В single-panel Collections:

```text
Enter
```

должен переключать single-panel view:

```text
Collections → Papers
```

с выбранной коллекцией.

В Papers при необходимости аналогично можно использовать команду перехода в Notes через `n`.

---

# 6. Дерево коллекций

## 6.1. Collapse / Expand

Добавить:

```rust
collapsed_collections: HashSet<Uuid>
```

Видимый список коллекций строить через:

```rust
build_visible_collection_items(...)
```

с учётом collapsed set.

Не хранить потомков скрытой ветки в отображаемом списке.

---

## 6.2. Tree-navigation

В Collections:

```text
l
```

- если коллекция закрыта → раскрыть;
- если уже раскрыта и содержит дочерние коллекции → перейти к первому ребёнку;
- если детей нет → ничего не делать.

```text
h
```

- если коллекция раскрыта → свернуть;
- если уже свернута → перейти к родителю.

Дополнительные Vim-команды:

```text
za       toggle текущей ветки
zM       свернуть всё
zR       раскрыть всё
```

Если `h/l` нужны глобально для смены панелей, tree-semantics должны применяться только при явном Tree Mode либо через альтернативные клавиши.

Рекомендуемый вариант:

```text
h/l          панели
Left/Right   collapse/expand
za/zM/zR     tree operations
```

чтобы не создавать конфликт.

---

# 7. Информация о позиции

Заголовки панелей должны показывать текущий контекст.

Примеры:

```text
Collections · 8/35

Papers · Machine Learning / Theory / SLT · Year↓ · 17/143

TOC · 31/186

Notes · HSE / Deep Learning 2026
```

Breadcrumb коллекции строится по `parent_id`.

Допускается кэш:

```rust
HashMap<Uuid, String>
```

---

# 8. Сортировка Papers

Ввести:

```rust
enum PaperSortField {
    Added,
    Year,
    Title,
    Author,
}

enum SortDirection {
    Asc,
    Desc,
}
```

Хоткей:

```text
S
```

циклически:

```text
Added ↓
Year ↓
Title ↑
Author ↑
```

Сортировка выполняется над уже загруженным `Vec<Paper>`.

SQLite schema менять не требуется.

После сортировки selection восстанавливается по UUID.

---

# 9. Paper ↔ Collection management

## 9.1. Collections picker

В Papers:

```text
c
```

открывает modal:

```text
Collections

[x] Machine Learning
[ ] Machine Learning / Optimization
[x] Deep Learning
[ ] Reading
[ ] Archive

> query

j/k      move
Space    toggle
Enter    apply
a        create collection
Esc      cancel
```

Использовать существующие:

```rust
CollectionRepo::add_paper(...)
CollectionRepo::remove_paper(...)
CollectionRepo::get_collections_for_paper(...)
```

Операция должна редактировать membership, а не выполнять move в одну коллекцию.

---

## 9.2. Fuzzy filtering

Picker коллекций должен поддерживать fuzzy filtering.

Для fuzzy matching рекомендуется:

```text
nucleo
```

или аналогичный быстрый matcher.

Fuzzy применяется к:

- collection picker;
- tag picker;
- Ctrl-p;
- command palette;
- вставке внутренних ссылок в Notes.

---

# 10. Generic Picker

Создать переиспользуемый компонент:

```rust
struct Picker<T> {
    items: Vec<T>,
    visible: Vec<usize>,
    selected: usize,
    query: String,
    checked: HashSet<Uuid>,
    multi_select: bool,
}
```

Picker должен поддерживать:

```text
typing     фильтр
j/k        навигация
↑/↓        навигация
Ctrl-d/u   page navigation
Space      toggle при multi-select
Enter      confirm
Esc        cancel
```

На этом компоненте позже строятся:

- collection picker;
- tag picker;
- quick open;
- command palette;
- Notes link picker.

---

# 11. Виртуальные коллекции

Добавить:

```text
All Papers
Recently Added
Unfiled
Untagged
```

## 11.1. Recently Added

Статьи сортируются по:

```text
created_at DESC
```

Рекомендуемая первая версия показывает последние добавленные статьи без жёсткого временного окна.

---

## 11.2. Unfiled

Статья входит в `Unfiled`, если для неё отсутствуют строки в:

```text
paper_collections
```

---

## 11.3. Untagged

Статья входит в `Untagged`, если `TagRepo` не возвращает ни одного тега.

---

# 12. Поиск

## 12.1. Tantivy остаётся основным поиском

Не заменять `/` fuzzy-matching.

`/` предназначен для содержательного поиска по:

- title;
- authors;
- abstract;
- body text;
- tags.

Fuzzy предназначен для навигационных picker'ов.

---

## 12.2. Навигация во время поиска

В режиме `/`:

```text
Ctrl-j / ↓    следующий результат
Ctrl-k / ↑    предыдущий результат
Enter          открыть выбранный результат
Esc            отменить поиск
```

Если нужен режим сохранения текущего фильтра:

```text
Ctrl-Enter
```

или альтернативная отдельная команда.

После Enter пользователь не должен делать дополнительный шаг для перехода к статье.

---

## 12.3. Search snippets

`SearchIndex::search()` уже возвращает `snippet`.

Не выбрасывать его после ранжирования.

Добавить:

```rust
struct SearchHitUi {
    score: f32,
    snippet: Option<String>,
}
```

И:

```rust
search_hits: HashMap<Uuid, SearchHitUi>
```

В Details показывать:

```text
SEARCH MATCH

"...the estimator achieves stochastic interpolation under..."
```

Совпавшие участки желательно выделять стилем Ratatui.

---

## 12.4. Query syntax

Поддержать:

```text
author:vaswani
title:attention
year:2017
tag:transformer
#transformer
```

Использовать возможности Tantivy QueryParser там, где это возможно.

---

## 12.5. История поиска

В режиме `/`:

```text
↑ / ↓
```

позволяют выбирать предыдущие запросы, если курсор результата не находится в navigation mode.

Хранить минимум последние 20 запросов.

Первая версия может быть только в RAM.

---

# 13. Quick Open

Добавить:

```text
Ctrl-p
```

Открывается глобальный fuzzy picker.

Ищет среди:

- Papers;
- Collections.

Пример:

```text
> atten

Papers
  Attention Is All You Need
  FlashAttention

Collections
  Transformers / Attention
```

Enter на Paper:

1. выбирает статью;
2. выбирает подходящий контекст;
3. переводит фокус в Papers.

Enter на Collection:

1. выбирает коллекцию;
2. переводит фокус в Papers.

Quick Open реализовать после стабилизации UUID-selection.

---

# 14. Jump history

Добавить историю навигации:

```rust
struct Location {
    collection: CollectionKey,
    paper_id: Option<Uuid>,
    toc_id: Option<Uuid>,
    context_view: ContextView,
}
```

Команды:

```text
Ctrl-o      назад
Ctrl-i      вперёд
```

В history добавлять только значимые переходы:

- переход между статьями;
- переход между коллекциями;
- search jump;
- quick open;
- открытие внутренней ссылки Notes.

Обычный `j/k` не должен создавать history entry на каждый шаг.

---

# 15. Yank / Clipboard

Ввести prefix namespace:

```text
yd    DOI
yb    BibTeX
yc    formatted citation
yt    title
ya    authors
yp    PDF path
```

После `y` статус-бар показывает:

```text
Yank: d DOI | b BibTeX | c citation | t title | a authors | p path
```

Добавить abstraction:

```rust
trait Clipboard {
    fn copy(&self, text: &str) -> Result<()>;
}
```

Первая реализация:

```text
ArboardClipboard
```

В будущем допускается:

```text
OSC52Clipboard
```

---

# 16. Duplicate import UX

Если импортируемый PDF уже существует по SHA-256:

не ограничиваться сообщением:

```text
Already imported
```

Нужно:

1. найти существующую статью;
2. выбрать её;
3. выбрать её коллекцию либо `All`;
4. показать:

```text
Already imported · selected existing paper
```

---

# 17. Session restore

После появления UUID-state добавить сохранение пользовательского состояния.

Хранить:

```text
selected collection
selected paper
active panel
ContextView
sort mode
collapsed collections
LayoutMode
```

Не сохранять transient modal state.

Допускается отдельный:

```text
state.json
```

или файл в config directory.

При старте:

- если UUID существует → восстановить;
- если объект удалён → использовать безопасный fallback.

---

# 18. Notes

## 18.1. Назначение

Notes предназначены для:

- учебных планов;
- reading lists;
- заметок по коллекции;
- заметок по конкретной статье;
- ссылок на конкретные главы книг;
- ссылок на конкретные страницы PDF;
- todo/checklist.

Пример:

```md
# Лекция 1

- [x] Прочитать Murphy §4.1
- [ ] Прочитать [§4.2 Attention](papyrus://paper/<paper-id>/toc/<toc-id>)
- [ ] Разобрать [страницы 120–135](papyrus://paper/<paper-id>/page/120)
```

---

## 18.2. Scope Notes

Первая версия:

```rust
enum NoteScope {
    Global,
    Collection(Uuid),
    Paper(Uuid),
}
```

На каждый scope одна Markdown-заметка.

Не делать сразу:

- папки Notes;
- несколько Notes на один scope;
- nested note hierarchy.

Markdown headings дают достаточную структуру первой версии.

---

## 18.3. База данных

Добавить migration:

```sql
CREATE TABLE notes (
    id          TEXT PRIMARY KEY NOT NULL,
    scope_type  TEXT NOT NULL,
    scope_id    TEXT,
    body_md     TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

Уникальность scope реализовать с учётом `NULL` для Global.

При необходимости Global можно хранить отдельным фиксированным scope key.

---

# 19. Markdown renderer

Поддержать ограниченный Markdown:

```text
# headings
**bold**
*italic*
`inline code`

- unordered lists
1. ordered lists

- [ ] tasks
- [x] completed tasks

[links](...)
```

Не требуется в первой версии:

- HTML;
- images;
- complex tables;
- embedded media.

Рекомендуемый parser:

```text
pulldown-cmark
```

Pipeline:

```text
Markdown
→ pulldown-cmark events
→ Vec<ratatui::text::Line>
→ Paragraph
```

Пример стилей:

```text
Heading       bold
Strong        bold
Emphasis      italic
Inline code   отдельный style
Link          underline
[x]           completed style
[ ]           normal task style
```

---

# 20. Notes editing

Не писать собственный полноценный multiline editor в первой версии.

Хоткей:

```text
e
```

в Notes:

1. создаёт временный `.md`;
2. запускает `$EDITOR`;
3. ждёт завершения editor process;
4. перечитывает файл;
5. сохраняет Markdown в SQLite;
6. удаляет временный файл.

Конфигурация:

```toml
[notes]
editor = "nvim"
```

Fallback:

```text
$EDITOR
```

При отсутствии обоих показывать понятную ошибку.

---

# 21. Notes tasks

В rendered Notes:

```text
Space
```

на строке:

```text
- [ ] ...
```

должен переключать:

```text
[ ] ↔ [x]
```

без открытия внешнего editor.

Изменение должно сохраняться в SQLite сразу.

---

# 22. Внутренние ссылки Notes

Поддержать URI:

```text
papyrus://paper/<paper-id>
papyrus://paper/<paper-id>/toc/<toc-id>
papyrus://paper/<paper-id>/page/<n>
papyrus://collection/<collection-id>
```

Поведение:

### paper

Выбирает статью в Papyrus.

### toc

Выбирает статью и соответствующий TOC node.

При `Enter` допускается сразу открыть PDF на странице TOC entry.

### page

Открывает соответствующий PDF через существующий `OpenAtPage`.

### collection

Переходит в указанную коллекцию.

---

# 23. Навигация внутри Notes

В режиме просмотра:

```text
j/k        scroll / move through interactive lines
Ctrl-d/u   half-page
gg/G       start/end

]l         next link
[l         previous link

Enter      открыть выбранную ссылку
Space      toggle task
e          edit in $EDITOR
```

Selection должен различать:

- обычный текст;
- task;
- link.

---

# 24. Notes link picker

После появления generic picker добавить команду вставки внутренних ссылок.

В Notes editing workflow можно предоставить отдельную команду:

```text
Ctrl-l
```

либо helper до открытия `$EDITOR`.

Picker:

```text
Insert link

> understanding

Understanding Deep Learning
  Chapter 4
  Chapter 5
  Chapter 6
```

Пользователь выбирает:

1. Paper;
2. optional TOC entry.

Papyrus генерирует:

```md
[Chapter 5](papyrus://paper/.../toc/...)
```

В первой версии допускается копирование готовой Markdown-ссылки в clipboard.

Это позволяет не интегрироваться напрямую с работающим `nvim`.

---

# 25. Notes как отдельная single-panel область

Это обязательная часть интерфейса.

При активной Notes:

```text
Ctrl-w
```

переходит в:

```text
┌─────────────────────────────────────────────────────────────┐
│ Notes · HSE / Deep Learning 2026                           │
│                                                             │
│ # Lecture 1                                                 │
│                                                             │
│ [x] Understanding Deep Learning §2                          │
│ [ ] Goodfellow §6                                           │
│ [ ] Attention Is All You Need                               │
│                                                             │
│ # Lecture 2                                                 │
│ ...                                                         │
└─────────────────────────────────────────────────────────────┘
```

В этом режиме:

- Collections скрыты;
- Papers скрыты;
- Notes занимают весь доступный viewport;
- status bar остаётся;
- search bar показывается только при активном note search, если он появится позже.

Повторный `Ctrl-w` возвращает исходный layout.

---

# 26. Tag picker

Существующее строковое редактирование тегов в дальнейшем заменить либо дополнить picker.

Поведение:

```text
T
```

↓

```text
Tags

[x] deep-learning
[ ] optimization
[x] theory

> trans_

Space toggle
a     create tag
Enter save
```

Использовать Generic Picker + fuzzy backend.

---

# 27. Command Palette

После Generic Picker добавить:

```text
:
```

Команды ищутся fuzzy-поиском.

Пример:

```text
> export

Export collection
Export library
Export paper metadata
```

Command palette предназначен для редких операций и не должен заменять основные хоткеи.

---

# 28. Reveal in Collection

Для статьи добавить command:

```text
Reveal in collection
```

Если статья находится в одной коллекции:

- перейти туда автоматически.

Если в нескольких:

- показать picker коллекций.

Это особенно полезно после глобального поиска.

---

# 29. Multi-select

Реализовать после базовых функций.

Предлагаемая модель:

```text
V
```

включает visual selection.

Навигация:

```text
j/k
```

расширяет диапазон.

Дополнительно допускается:

```text
Space
```

для mark/unmark отдельных элементов.

Batch operations:

- добавить/убрать коллекции;
- добавить/убрать tags;
- export;
- delete.

Не реализовывать batch через последовательный вызов UI actions.

Для массовых операций должен появиться отдельный batch-слой.

---

# 30. Trash

Реализовать отдельной фазой.

Не считать Trash XS-функцией.

Нужно определить семантику:

- показывать ли статью в Tantivy;
- показывать ли в All Papers;
- сохранять ли memberships;
- сохранять ли tags;
- включать ли в export;
- duplicate import;
- restore;
- permanent delete.

Предпочтительная модель:

```text
d             move to Trash
Trash         virtual collection
r             restore
D             delete permanently
```

После trash обычные поиски не должны показывать документ.

---

# 31. Saved Searches

Отложенная функция.

Пользователь должен иметь возможность сохранить поисковый запрос как виртуальную коллекцию.

Пример:

```text
title:diffusion year:2025
```

↓

```text
Saved
└── Recent Diffusion
```

Saved search хранит query, а не список статей.

---

# 32. Status bar / prefix hints

Status bar должен быть context-aware.

Пример Papers:

```text
[Papers] j/k Nav | c Collections | S Sort | y Yank | / Search | Ctrl-p Open | Ctrl-w Focus
```

После `y`:

```text
Yank: d DOI | b BibTeX | c citation | t title | a authors | p path
```

После `z` в Collections:

```text
Fold: a toggle | M collapse all | R expand all
```

Это заменяет необходимость отдельного which-key framework.

---

# 33. Конфликты хоткеев

Текущие занятые команды учитывать обязательно.

Известные:

```text
s      search alias
m      metadata import
t      fullscreen TOC
T      tags
e      metadata / edit
```

Предпочтения:

```text
/      основной search
S      sort
c      collections
Ctrl-p quick open
Ctrl-w single-panel toggle
y...   yank namespace
z...   tree namespace
```

Если `s` больше не нужен как alias `/`, допускается освободить его позднее, но существующее поведение не ломать без отдельного решения.

---

# 34. Рекомендуемая структура модулей

Не продолжать раздувать `dispatch.rs`.

Добавить отдельные модули:

```text
app/
  navigation.rs
  selection.rs
  sorting.rs
  picker.rs
  collections.rs
  virtual_collections.rs
  notes.rs
  session.rs
  jump_history.rs
  clipboard.rs
```

UI:

```text
ui/
  notes.rs
  picker.rs
```

Core:

```text
db/
  note_repo.rs
```

Markdown renderer допускается держать в TUI:

```text
notes/markdown.rs
```

если core не использует Markdown representation.

---

# 35. Порядок реализации

## Phase A. Stable selection

1. `CollectionKey`.
2. UUID-based selection.
3. `last_paper_by_collection`.
4. `last_toc_by_paper`.
5. восстановление UUID после reload/reorder.
6. тесты на reorder, delete и fallback selection.

---

## Phase B. Navigation

7. `h/l`.
8. `Enter` Collections → Papers.
9. `gg/G`.
10. `Ctrl-u/Ctrl-d`.
11. numeric counts.
12. Breadcrumb.
13. `x/n` indicators.
14. сортировка `S`.
15. single-panel mode `Ctrl-w`.

---

## Phase C. Collections

16. Generic picker base.
17. `c` collection membership.
18. collapse/expand.
19. `za/zM/zR`.
20. `Unfiled`.
21. `Untagged`.
22. `Recently Added`.

---

## Phase D. Notes v1

23. `notes` migration.
24. `NoteRepo`.
25. Global / Collection / Paper scope.
26. Notes context view.
27. Markdown renderer.
28. `$EDITOR` editing.
29. task toggling.
30. internal `paper/page/toc/collection` links.
31. Notes single-panel mode.

---

## Phase E. Search

32. navigation while searching.
33. Enter opens search result.
34. search snippets.
35. search history.
36. field syntax.

---

## Phase F. Fuzzy/navigation tools

37. nucleo backend.
38. `Ctrl-p`.
39. tag picker.
40. fuzzy collection picker.
41. link insertion picker.
42. `:` command palette.

---

## Phase G. Workflow

43. yank namespace.
44. duplicate import → jump.
45. reveal in collection.
46. session restore.
47. `Ctrl-o/Ctrl-i`.

---

## Phase H. Larger features

48. multi-select.
49. Trash.
50. saved searches.

---

# 36. Acceptance criteria

## Selection

После сортировки выбранная статья остаётся выбранной по UUID.

После collapse/expand текущая коллекция не должна случайно менять identity.

После DB reload selection восстанавливается по UUID.

Удалённый объект не приводит к panic или invalid index.

---

## Navigation

Работают:

```text
j/k
gg/G
5j
10G
Ctrl-u/d
h/l
```

во всех основных list-based панелях.

---

## Single-panel mode

`Ctrl-w`:

- из Collections показывает только Collections;
- из Papers показывает только Papers;
- из Notes показывает только Notes;
- повторное нажатие возвращает multi-panel layout;
- selection и scroll сохраняются.

---

## Collections

`c` позволяет:

- увидеть текущие memberships;
- добавить статью в несколько коллекций;
- убрать статью из коллекций;
- сохранить изменения одной операцией.

---

## Search

`/` использует Tantivy.

Во время поиска можно выбрать результат и открыть его без выхода в отдельную стадию навигации.

Snippet полнотекстового совпадения показывается пользователю.

---

## Notes

Пользователь может:

1. открыть Notes для коллекции;
2. создать Markdown;
3. открыть его в `$EDITOR`;
4. сохранить;
5. видеть отрендеренный Markdown;
6. переключить checkbox через `Space`;
7. открыть внутреннюю ссылку на статью;
8. открыть ссылку на TOC chapter;
9. открыть PDF на указанной странице;
10. развернуть Notes на весь терминал через `Ctrl-w`.

---

## Session

После перезапуска приложения восстанавливаются:

- collection;
- paper;
- panel;
- ContextView;
- sort;
- collapsed branches.

---

# 37. Что не делать в первой реализации

Не добавлять одновременно:

- полноценный встроенный Markdown editor;
- полноценные Vim operators;
- Vim registers;
- text objects;
- fuzzy вместо Tantivy;
- сложную hierarchy Notes;
- изображения в Markdown;
- batch API до появления multi-select;
- Trash как простой `trashed_at` без проработки поиска/export/restore.

Главная линия развития Papyrus:

```text
устойчивое состояние
→ быстрая клавиатурная навигация
→ удобная организация библиотеки
→ Notes как reading/workflow layer
→ улучшенный полнотекстовый поиск
→ общие fuzzy picker-инструменты
→ batch/workflow функции
```
