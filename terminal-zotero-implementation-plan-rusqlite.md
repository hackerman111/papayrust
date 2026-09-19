# Terminal Zotero (`papyrus`) — план реализации

> Рабочее кодовое имя — `TocToc-papayrust`. Заменить на финальное при инициализации репозитория.

**Ревизия 3.** Относительно первой версии плана изменено два принципиальных момента:

1. **Хранилище переведено на SQLite через `rusqlite`**. В MVP используется `rusqlite` с фичей `bundled`, чтобы версия SQLite была фиксирована зависимостью проекта и не зависела от системного пакета. Это означает наличие C-компиляции SQLite через `libsqlite3-sys`/`cc` при сборке, поэтому требование «только pure Rust зависимости» для слоя хранения снимается.
2. Добавлена **фича импорта оглавления в конкретную статью** — массовая загрузка/повторное извлечение оглавления из внешнего источника, в дополнение к ручному постатейному вводу из ревизии 1.

Правило кумулятивной регрессии и структура фаз из §0 первой версии сохраняются без изменений.

---

## 0. Инженерный процесс

- `AGENTS.md` в духе [[calendar-tui]]: владение модулями, дисциплина borrowing/unsafe, конкурентность, стиль.
- **Правило перехода между фазами** не изменилось: фаза закрыта только при (а) тестах на новый код, (б) зелёном кумулятивном регрессионном наборе, (в) чистых `cargo build --release` и `cargo clippy --all-targets -- -D warnings`.
- Workspace из трёх крейтов (см. ниже) — не изменился относительно ревизии 1; изменилось содержимое `db`.

```text
papyrus/
├── Cargo.toml
├── AGENTS.md
├── crates/
│   ├── papyrus-core/
│   │   ├── db/            # SQLite через rusqlite
│   │   ├── pdf/
│   │   ├── search/
│   │   ├── export/
│   │   ├── toc/
│   │   │   ├── model.rs
│   │   │   ├── extract.rs   # авто-извлечение /Outlines
│   │   │   ├── import.rs    # НОВОЕ: text/json/pdf-импорт
│   │   │   ├── editor_ops.rs
│   │   │   └── embed.rs
│   │   └── opener.rs
│   ├── papyrus-tui/
│   └── papyrus-cli/
└── tests/
```

---

## 1. Стек

| Область                        | Крейт                                                     | Фичи / оговорки                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------------ | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| TUI                            | `ratatui`, `crossterm`                                    | default; чистый Rust подтверждён                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| **Хранилище**                  | `rusqlite`                                                | `default-features = false`, `features = ["bundled", "backup", "functions"]`; SQLite поставляется вместе с приложением через `libsqlite3-sys`                                                                                                                                                                                                                                                                                                                                                                                               |
| Полнотекстовый поиск           | `tantivy`                                                 | **default-features = false**, `features = ["mmap", "stopwords", "lz4-compression"]`. По умолчанию сжатие индекса — `lz4_flex` (чистый Rust); `zstd-compression` — опциональная фича на C-биндинге `zstd-sys`, её не включать. **Важно для аудита**: в части версий tantivy в дереве фич встречается `columnar-zstd-compression`, местами помеченная как часть default-набора — перед фиксацией версии в Фазе 0 явно проверить через `cargo tree`, что при отключённых default-features она не тянется, иначе придётся пере-пиновать версию |
| PDF-метаданные, запись outline | `lopdf`                                                   | исторически чистый Rust; подтвердить в ходе аудита Фазы 0 (не проверялся отдельным поиском в рамках этой ревизии)                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Извлечение текста для индекса  | `pdf-extract`                                             | подтверждено: зависимости — `adobe-cmap-parser`, `encoding`, `euclid`, `linked-hash-map`, `lopdf`, `postscript`, `type1-encoding-parser`, `unicode-normalization` — ни одного `-sys`-крейта                                                                                                                                                                                                                                                                                                                                                |
| ZIP                            | `zip`                                                     | **default-features = false**, `features = ["deflate"]`. По умолчанию у крейта `zip` включены `bzip2` и `zstd` — оба тянут C-биндинги (`bzip2-sys`, `zstd-sys`); их не включать. `deflate` использует `flate2` со стандартным бэкендом `miniz_oxide` — чистый безопасный Rust                                                                                                                                                                                                                                                               |
| Хеши для дедупликации          | `sha2`                                                    | default (без фичи `asm`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Идентификаторы                 | `uuid` (v7)                                               | default                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Фоновые задачи                 | `tokio`                                                   | чистый Rust                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Конфиг / CLI / ошибки / логи   | `serde`, `toml`, `clap`, `thiserror`, `anyhow`, `tracing` | чистый Rust                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |

**Компромисс.** `rusqlite` не является pure-Rust зависимостью: при `features = ["bundled"]` SQLite собирается из C-исходников через `libsqlite3-sys`. Взамен приложение получает зрелую транзакционную модель SQLite, `FOREIGN KEY`, уникальные ограничения, индексы, ad-hoc SQL для диагностики и стандартные механизмы backup/snapshot. Весь SQL по-прежнему изолируется в репозиториях (`PaperRepo`, `CollectionRepo`, `TocRepo`), чтобы UI и бизнес-логика не зависели от конкретных запросов.

**SQL разрешён как часть основного хранилища.** Аналитические и диагностические запросы можно выполнять напрямую по SQLite, но код приложения не должен обходить репозиторный слой для мутаций данных.

---

## 2. Модель данных

Схема хранится в SQLite. При каждом открытии соединения приложение выполняет:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
```

DDL первой версии:

```sql
CREATE TABLE papers (
    id                  TEXT PRIMARY KEY NOT NULL,
    file_path           TEXT NOT NULL UNIQUE,
    content_hash        TEXT NOT NULL UNIQUE,

    title               TEXT,
    authors             TEXT,
    year                INTEGER,
    journal             TEXT,
    doi                 TEXT,
    abstract_text       TEXT,

    text_path           TEXT,
    annotated_pdf_path  TEXT,
    toc_embedded_at     TEXT,

    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE TABLE collections (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    parent_id   TEXT,
    FOREIGN KEY (parent_id)
        REFERENCES collections(id)
        ON DELETE SET NULL
);

CREATE TABLE paper_collections (
    paper_id       TEXT NOT NULL,
    collection_id  TEXT NOT NULL,

    PRIMARY KEY (paper_id, collection_id),

    FOREIGN KEY (paper_id)
        REFERENCES papers(id)
        ON DELETE CASCADE,

    FOREIGN KEY (collection_id)
        REFERENCES collections(id)
        ON DELETE CASCADE
);

CREATE TABLE toc_entries (
    id            TEXT PRIMARY KEY NOT NULL,
    paper_id      TEXT NOT NULL,
    parent_id     TEXT,
    title         TEXT NOT NULL,
    page_number   INTEGER NOT NULL CHECK (page_number >= 1),
    order_index   INTEGER NOT NULL,
    source        TEXT NOT NULL CHECK (source IN ('auto', 'manual', 'imported')),
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,

    FOREIGN KEY (paper_id)
        REFERENCES papers(id)
        ON DELETE CASCADE,

    FOREIGN KEY (parent_id)
        REFERENCES toc_entries(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_paper_collections_collection
    ON paper_collections(collection_id);

CREATE INDEX idx_toc_entries_paper
    ON toc_entries(paper_id);

CREATE INDEX idx_toc_entries_parent
    ON toc_entries(parent_id);

CREATE INDEX idx_toc_entries_siblings
    ON toc_entries(paper_id, parent_id, order_index);
```

`Uuid` хранится как canonical lowercase string. На уровне Rust репозитории преобразуют его в/из `uuid::Uuid`; UI и action-слой не работают со строковым представлением идентификатора напрямую.

`TocSource` остаётся Rust-enum:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TocSource {
    Auto,
    Manual,
    Imported,
}
```

Репозиторный слой отвечает за отображение `TocSource` в значения SQLite `auto` / `manual` / `imported`.

**Каскадные удаления.** SQLite выполняет каскады декларативно через `ON DELETE CASCADE`, при обязательном `PRAGMA foreign_keys = ON` на каждом соединении:

- удаление `papers` автоматически удаляет связанные `paper_collections` и `toc_entries`;
- удаление `collections` автоматически удаляет только связи из `paper_collections`, не сами статьи;
- удаление родительского `toc_entries` автоматически удаляет всё поддерево через self-referencing foreign key.

Все мутации проходят через `PaperRepo` / `CollectionRepo` / `TocRepo`. Прямые `DELETE`/`UPDATE` из TUI и CLI запрещены архитектурно.

**Миграции.** Используется таблица `schema_migrations`:

```sql
CREATE TABLE schema_migrations (
    version     INTEGER PRIMARY KEY NOT NULL,
    applied_at  TEXT NOT NULL
);
```

Каждая миграция — отдельный SQL-файл или Rust-константа с монотонным номером версии. Миграция выполняется внутри транзакции; после успешного применения версия добавляется в `schema_migrations`. Повторное открытие БД не должно повторно применять уже выполненные миграции.

---

## 3. Action-слой

```rust
enum Action {
    NextPanel, PreviousPanel,
    MoveUp, MoveDown,
    Open,
    OpenAtPage(u32),
    Search,
    ExportLibrary,
    EditMetadata,

    OpenToc,
    TocAddEntry { parent_id: Option<Uuid> },
    TocEditEntry { id: Uuid },
    TocDeleteEntry { id: Uuid },       // каскадно удаляет детей, см. §2
    TocIndent { id: Uuid },
    TocOutdent { id: Uuid },
    TocMoveUp { id: Uuid },
    TocMoveDown { id: Uuid },
    TocEmbed,
    TocImport { source: TocImportSource },   // НОВОЕ

    Quit,
}

enum TocImportSource {
    PdfOutline,             // повторно прогнать extract:: из Фазы 2
    TextFile(std::path::PathBuf),
    JsonFile(std::path::PathBuf),
}
```

---

## 4. Конфигурация

`database_path` снова указывает на SQLite-файл:

```toml
library_path = "/home/user/papers"
database_path = "/home/user/papers/.library/library.db"

[pdf]
viewer = "zathura"
page_open_template = "zathura --page={page} {path}"

[toc]
prefer_annotated_copy = true
annotated_dir = ".library/annotated"
auto_extract_on_import = true

[export]
directory = "/home/user/backups"
compression = "deflate"
```

---

## 5. Ручное оглавление PDF

### 5.1 Источники записей

`TocSource` теперь настоящий Rust-enum (§2), а не текстовый `CHECK`: `Auto` (авто-извлечение при первичном импорте PDF), `Manual` (постатейный ввод в TUI), `Imported` (массовая загрузка, см. §5.6). Все три живут в одном дереве `toc_entries`, различаясь бейджем в UI.

### 5.2 Взаимодействие в TUI

Без изменений относительно ревизии 1 — режим `t` из панели Metadata, клавиши `a`/`e`/`d`/`H`/`L`/`K`/`J`/`E`. Добавлена клавиша `i` — импорт (§5.6).

### 5.3 Материализация оглавления в PDF

Без изменений по сути: `lopdf` строит дерево `/Outlines` из `toc_entries`, копия пишется атомарно (`tmp` → `rename`) в `annotated_dir`, оригинал по `file_path` не трогается ни при каких условиях — это по-прежнему инвариант, а не деталь реализации.

### 5.4 Переход по странице без embed

Без изменений.

### 5.5 Учёт в экспорте

Без изменений по структуре `manifest.json`; консистентная копия БД создаётся через SQLite Online Backup API, доступный в `rusqlite::backup`, и кладётся в `.library/library.db` внутри архива. Простое копирование открытого `.db`-файла запрещено, особенно при WAL-режиме.

### 5.6 Импорт оглавления в конкретную статью (новое)

Формулировку «добавь возможность импортировать оглавление в конкретный файл» я понимаю как: подготовить оглавление во внешнем источнике и одним действием применить («импортировать») его целиком к конкретной статье в библиотеке — в дополнение к поштучному ручному вводу из §5.2. Ниже — полная спецификация под это понимание; если имелось в виду что-то более узкое (например, только повторное извлечение из самого PDF), она включена как один из трёх поддерживаемых источников и ничего лишнего не потребует выкидывать.

**Источники (`TocImportSource`):**

1. **`PdfOutline`** — повторный запуск того же кода извлечения `/Outlines`, что и при первичном импорте (Фаза 2). Полезно для статей, добавленных до появления этой фичи, или если исходный outline на момент первичного импорта был повреждён.
2. **`TextFile`** — простой построчный формат с отступами, рассчитанный на то, что оглавление можно быстро набрать руками или скопировать из содержания книги/статьи:

```text
Introduction	1
Background	2
	Scaled Dot-Product Attention	3
	Multi-Head Attention	4
Model Architecture	5
Results	9
```

Разделитель заголовка и номера страницы — последний `\t` в строке (пробелы допустимы внутри заголовка). Уровень вложенности — число ведущих табов (либо, по конфигурируемому альтернативному варианту, кратное 4 пробелам; смешение табов и пробелов в одном файле — ошибка импорта). Уровень не может «прыгать» больше чем на 1 относительно предыдущей строки.

3. **`JsonFile`** — та же форма, что уже используется в `manifest.json` (§5.5), что даёт симметричный экспорт/импорт:

```json
[
  { "title": "Introduction", "page": 1, "level": 0 },
  { "title": "Background", "page": 2, "level": 0 },
  { "title": "Scaled Dot-Product Attention", "page": 3, "level": 1 }
]
```

**Конвейер валидации (`papyrus-core::toc::import`), одинаковый для всех трёх источников:**

1. Разобрать источник целиком в `Vec<PendingTocEntry>` (заголовок, страница, уровень) — ничего не пишется в БД на этом шаге;
2. Получить число страниц PDF через `lopdf` (`Document::get_pages().len()`);
3. Проверить каждую запись: заголовок непустой после `trim()`; `1 <= page_number <= page_count`; уровни образуют корректное дерево (без скачков более чем на 1 вглубь);
4. При любой ошибке вернуть `ImportError::Line { line_no, reason }` (для текста/JSON) или `ImportError::EmptyOutline` (для `PdfOutline`) — **ничего не записывается**, весь импорт — всё-или-ничего;
5. Если валидация прошла — одна `rw_transaction`: режим по умолчанию **replace** (существующие `toc_entries` статьи удаляются и заменяются новыми), опциональный режим **merge** (новые записи добавляются как дополнительные записи верхнего уровня, существующие не трогаются);
6. `updated_at` статьи и записей обновляется, чтобы уже существующая аннотированная копия (§5.3) сразу пометилась в UI как устаревшая до повторного `E`/`app toc embed`.

**TUI.** Клавиша `i` в режиме TOC-редактора открывает мини-форму выбора источника → путь к файлу (для текстового/JSON) → **предпросмотр разобранного дерева** перед подтверждением записи, чтобы неудачный парсинг не привёл к применению мусора без возможности взглянуть на результат заранее.

**CLI.**

```bash
app toc import <paper-id> --from-pdf
app toc import <paper-id> --from-text toc.txt
app toc import <paper-id> --from-json toc.json
app toc import <paper-id> --from-text toc.txt --merge

# симметричный экспорт для повторного использования/бэкапа/переноса между похожими статьями
app toc export <paper-id> --to toc.json
```

---

## 6. Фазы реализации

Правило кумулятивной регрессии из §0 действует на каждую фазу без повторного упоминания.

### Фаза 0 — Bootstrap

Как в ревизии 1, плюс явная фиксация версий `rusqlite`/SQLite/`tantivy`/`zip`. Для `rusqlite` включить `bundled` и проверить сборку на всех поддерживаемых target'ах CI.

**Тесты.** `RT-01` конфиг; `RT-02` отсутствующий конфиг; `RT-03` дефолты `[export]`; **`RT-03a`** — smoke-test создаёт SQLite БД через `rusqlite`, применяет начальную миграцию и успешно выполняет `PRAGMA foreign_keys`/`journal_mode`.

### Фаза 1 — Хранилище и CRUD

**Цель.** Таблицы `papers`/`collections`/`paper_collections`/`toc_entries` поверх SQLite (`rusqlite`), репозитории `PaperRepo`/`CollectionRepo`/`TocRepo`, миграции и декларативные каскады (§2).

**DoD.** Полный CRUD, каскады работают через `FOREIGN KEY ... ON DELETE`, уникальные ограничения (`file_path`, `content_hash`, `name` коллекции) отклоняют дубликаты на уровне SQLite.

**Тесты.**

- `RT-04` — SQL-миграции применяются по порядку и идемпотентно при повторном открытии БД;
- `RT-05` — CRUD статьи;
- `RT-06` — CRUD коллекции, rename, delete не удаляет статьи;
- `RT-07` — статья в нескольких коллекциях через `paper_collections`, `remove-from-collection` не трогает PDF и другие связи;
- `RT-08` — удаление статьи через `PaperRepo::delete` приводит к каскадному удалению связей и `toc_entries` средствами SQLite;
- `RT-08a` — вставка дубликата по `UNIQUE` (`file_path` или `content_hash`) отклоняется SQLite, а не только проверкой в бизнес-коде.

### Фаза 2 — Импорт PDF и метаданные

Без изменений по содержанию относительно ревизии 1 (`RT-09`–`RT-13`), дедупликация опирается на `UNIQUE(content_hash)` и запрос по `content_hash`.

### Фаза 3 — Каркас TUI

Без изменений (`RT-14`–`RT-16`).

### Фаза 4 — Внешний вьюер

Без изменений (`RT-17`–`RT-19`).

### Фаза 5 — Полнотекстовый поиск

Без изменений по функциональности; при написании `search/index.rs` — жёстко закрепить `default-features = false` для `tantivy` (§1) прежде, чем писать код индексации, а не постфактум. Тесты `RT-20`–`RT-24` без изменений.

### Фаза 6 — Редактирование метаданных

Без изменений (`RT-25`–`RT-26`).

### Фаза 7 — Ручное оглавление PDF и импорт (расширена)

**Цель.** CRUD записей, reorder/reparent, материализация в PDF (ревизия 1) **плюс** массовый импорт из трёх источников (§5.6).

**DoD.** Полный цикл ручного ввода → embed, **и** полный цикл импорта (все три источника, оба режима replace/merge) → embed, с идентичным результатом для эквивалентных данных.

**Тесты.**

- `RT-27` — добавление записи верхнего уровня через `Action`;
- `RT-28` — добавление дочерней записи;
- `RT-29` — indent/outdent корректно меняет `parent_id`, включая перепривязку внуков при outdent родителя с детьми;
- `RT-30` — reorder (`K`/`J`) стабильно меняет `order_index` относительно siblings;
- `RT-31` — удаление записи с дочерними каскадно удаляет поддерево через self-referencing `FOREIGN KEY ... ON DELETE CASCADE`;
- `RT-32` — embed: сгенерированный PDF имеет `/Outlines` с ожидаемыми заголовками/страницами, парсится обратно через `lopdf`;
- `RT-33` — embed не меняет байты оригинала (сравнение хеша до/после);
- `RT-34` — повторный embed после правки обновляет аннотированную копию и `toc_embedded_at`, UI помечает копию устаревшей до этого момента;
- `RT-35` — записи `source = Auto` редактируются, не теряя признак источника при частичном обновлении полей;
- `RT-36` — `OpenAtPage` без embed открывает оригинал с корректной подстановкой `{page}`;
- `RT-37` — импорт текстового формата с вложенностью строит корректное дерево;
- `RT-38` — импорт JSON-формата даёт тот же результат, что текстовый импорт эквивалентного дерева;
- `RT-39` — импорт с `page_number` больше числа страниц PDF отклоняется целиком, БД не меняется;
- `RT-40` — импорт с недопустимым скачком уровня вложенности возвращает номер строки, БД не меняется;
- `RT-41` — режим `--merge` добавляет записи, не удаляя существующие; без флага — полная замена;
- `RT-42` — `--from-pdf` повторно извлекает `/Outlines`, записи помечаются `Imported` (не `Auto`); `app toc export` → `app toc import` того же файла даёт идентичное (с точностью до id/времени) дерево.

### Фаза 8 — ZIP-экспорт

Функциональность без изменений; консистентный снапшот БД создаётся через `rusqlite::backup` / SQLite Online Backup API.

**Тесты.** `RT-43` одна коллекция; `RT-44` статья в нескольких коллекциях; `RT-45` коллизия имён файлов; `RT-46` пустая коллекция; `RT-47` отсутствующий PDF; `RT-48` Unicode; `RT-49` защита от `../`/абсолютных путей; `RT-50` консистентный SQLite hot-backup при параллельной записи; `RT-51` валидность `manifest.json` (включая `toc_entries`); `RT-52` повторное открытие готового ZIP; `RT-53` атомарность при прерывании экспорта; `RT-54` поведение `--force`; `RT-55` аннотированные копии PDF (`.annotated.pdf`) попадают в архив только при актуальности.

### Фаза 9 — Унификация CLI

Без изменений по идее. **Тесты.** `RT-56` `app add` эквивалентен добавлению через TUI; `RT-57` варианты `app export`/`--force`; `RT-58` `app toc embed` даёт байт-в-байт идентичный файл CLI и TUI-путём.

### Фаза 10 — Надёжность и `app doctor`

**DoD.** Ни один сценарий из исходного п.16 не паникует; `app doctor` выполняет `PRAGMA quick_check` для обычной проверки и `PRAGMA integrity_check` в полном режиме, дополнительно проверяя внешние файлы и логические инварианты.

**Тесты.** `RT-59`–`RT-69` — по одному тесту на каждый из 11 пунктов отказоустойчивости (битый PDF, отсутствующие метаданные, удалённый PDF, удалённый TXT, битый Tantivy-индекс, пустая коллекция, пустая библиотека, ошибка создания ZIP, недостаток места, коллизия имён, нестандартные символы); `RT-70` — `app doctor` находит "мёртвые" `annotated_pdf_path`, указывающие на несуществующий файл, и нарушения ссылочной целостности/логических инвариантов после стороннего вмешательства в `.db`; `RT-71` — `app doctor` выполняет `PRAGMA integrity_check` и корректно репортит повреждение.

### Фаза 11 — Финальная регрессия и приёмка

Прогон `RT-01`…`RT-71`, затем ручной сценарий из ревизии 1 (создание коллекций, добавление статей, поиск, TOC-редактор с embed, перезапуск, экспорт, повторное открытие ZIP) **плюс**:

7. Взять готовый текстовый файл оглавления, выполнить `app toc import <id> --from-text toc.txt`, убедиться, что дерево совпадает с ожидаемым и что последующий `E`/`app toc embed` даёт PDF с тем же самым оглавлением, что и при постатейном ручном вводе для идентичных данных.

---

## 7. Сводная матрица регрессии

| Фаза | Новые тесты                            | Обязательно перепрогонять начиная с фазы |
| ---- | -------------------------------------- | ---------------------------------------- |
| 0    | RT-01–03, RT-03a                       | 1                                        |
| 1    | RT-04–08, RT-08a                       | 2                                        |
| 2    | RT-09–13                               | 3                                        |
| 3    | RT-14–16                               | 4                                        |
| 4    | RT-17–19                               | 5                                        |
| 5    | RT-20–24                               | 6                                        |
| 6    | RT-25–26                               | 7                                        |
| 7    | RT-27–42                               | 8                                        |
| 8    | RT-43–55                               | 9                                        |
| 9    | RT-56–58                               | 10                                       |
| 10   | RT-59–71                               | 11                                       |
| 11   | ручной приёмочный сценарий (7 пунктов) | — (финал)                                |

---

## 8. Не входит в этот план (non-goals)

Как в ревизии 1: облачная синхронизация, Zotero API, браузерное расширение, PDF-аннотации (кроме оглавления), собственный PDF-рендерер, OCR, LLM/embeddings/semantic search, совместная работа, автозагрузка статей, синхронизация BibTeX, обратный импорт ZIP.

---

## 9. Бэклог за пределами этого документа

Без изменений относительно ревизии 1 (приоритет `.tex` в поиске, экспорт в Excel, диф-перенос части библиотеки, поиск по ISBN с открытием в браузере, «неотерминальная» тема). Кросс-компиляция под Android (`cargo-ndk`) остаётся вне рамок документа; при ней нужно отдельно учесть сборку bundled SQLite для Android target.
