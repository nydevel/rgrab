# Архитектура rgrab

## Обзор

rgrab — легковесный observability-бэкенд на Rust. Принимает, хранит и отдаёт логи и distributed traces. Предоставляет Loki-совместимый API для интеграции с существующими инструментами (Grafana, promtail и др.).

## Структура проекта

```
rgrab/
├── Cargo.toml              # workspace root
├── CLAUDE.md               # coding standards
├── docs/                   # документация
└── crates/
    ├── common/             # модели данных, типы, парсеры
    ├── storage/            # слой хранения (RocksDB)
    ├── collector/          # API приёма данных (ingest)
    ├── web/                # API запросов + Loki API
    └── server/             # единый бинарник
```

## Крейты

### common

Общие типы данных, используемые всеми остальными крейтами.

| Модуль             | Назначение                                                |
|--------------------|-----------------------------------------------------------|
| `log.rs`           | `LogEntry`, `LogLevel` — структура лог-записи             |
| `span.rs`          | `Span`, `SpanStatus`, `SpanEvent` — distributed tracing   |
| `loki.rs`          | Loki-совместимые request/response типы, `LabelMatcher`     |
| `label_selector.rs`| Парсер label selector синтаксиса `{key="val", k2!="v2"}`  |

### storage

Персистентное хранилище на RocksDB.

**RocksStore** — основная реализация. Использует две column families:

- **logs** — лог-записи. Ключ: `[8B timestamp_nanos BE][8B sequence BE]` (16 байт). Естественная хронологическая сортировка.
- **spans** — спаны. Ключ: `"{trace_id}:{span_id}"`. Позволяет prefix-итерацию по trace_id.

Значения хранятся как JSON (`serde_json`).

Все публичные методы — `async`, внутри используют `tokio::task::spawn_blocking` чтобы не блокировать async runtime.

**InMemoryStore** — in-memory реализация (Vec + RwLock), сохранена для тестов/отладки.

### collector

Сервис приёма данных. Эндпоинты:

- `POST /v1/logs` — приём массива `LogEntry`
- `POST /v1/traces` — приём массива `Span`

### web

Сервис запросов. Два набора API:

**Native API** (`api.rs`):
- `GET /api/logs?limit=N`
- `GET /api/traces?trace_id=ID&limit=N`

**Loki-совместимый API** (`loki_api.rs`):
- `POST /rgrab/api/v1/push`
- `GET /rgrab/api/v1/query`
- `GET /rgrab/api/v1/query_range`
- `GET /rgrab/api/v1/labels`
- `GET /rgrab/api/v1/label/{name}/values`

### server

Единый бинарник, объединяющий collector + web + loki_api в одном процессе. Рекомендуемый способ запуска.

Необходим потому что RocksDB блокирует директорию БД — два процесса не могут открыть одну БД одновременно.

## Поток данных

```
                    ┌──────────────────┐
                    │   Приложения     │
                    │ (promtail, SDK)  │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
     POST /v1/logs   POST /v1/traces  POST /rgrab/api/v1/push
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼─────────┐
                    │   RocksStore     │
                    │  (column families│
                    │   logs / spans)  │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
     GET /api/logs    GET /api/traces  GET /rgrab/api/v1/query*
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼─────────┐
                    │    Grafana /     │
                    │   curl / UI     │
                    └──────────────────┘
```

## Ключевые решения

1. **RocksDB** — встроенная БД, не требует отдельного сервера. Персистентность из коробки.
2. **Единый бинарник** — упрощает деплой, избегает проблемы блокировки БД.
3. **Loki-совместимый API** — свои пути (`/rgrab/api/v1/...`), но формат ответов идентичен Loki. Позволяет использовать Grafana как UI.
4. **Label-based filtering** — логи фильтруются по лейблам, как в Loki. Поддержка `=`, `!=`, `=~`, `!~`.
5. **Scan-based queries** — v1 использует полный скан с фильтрацией. Для малых и средних объёмов данных этого достаточно.

## Стек технологий

| Компонент       | Технология                |
|-----------------|---------------------------|
| Язык            | Rust (edition 2024)       |
| Async runtime   | Tokio                     |
| HTTP framework  | Axum 0.8                  |
| Хранилище       | RocksDB 0.22              |
| Сериализация    | serde / serde_json        |
| Логирование     | tracing / tracing-subscriber |
| Error handling  | anyhow                    |
