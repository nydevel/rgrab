# Архитектура rgrab

## Обзор

rgrab — легковесный observability-бэкенд на Rust. Принимает, хранит и отдаёт логи и distributed traces. Предоставляет Loki-совместимый API для интеграции с Grafana, поддерживает приём данных в формате OpenTelemetry (OTLP). Включает TUI-клиент для просмотра логов и трейсов в терминале.

## Структура проекта

```
rgrab/
├── Cargo.toml              # workspace root
├── CLAUDE.md               # coding standards
├── docs/                   # документация
├── packaging/              # systemd, config, scripts
│   ├── config/rgrab.toml   # конфигурация по умолчанию
│   ├── systemd/            # systemd unit
│   └── scripts/            # postinst, prerm
└── crates/
    ├── common/             # модели данных, типы, парсеры
    ├── storage/            # слой хранения (RocksDB)
    ├── collector/          # API приёма данных (ingest)
    ├── web/                # API запросов + Loki API
    ├── server/             # единый серверный бинарник
    └── tui/                # TUI-клиент (rgrab-tui)
```

## Крейты

### common

Общие типы данных, используемые всеми остальными крейтами.

| Модуль             | Назначение                                                |
|--------------------|-----------------------------------------------------------|
| `log.rs`           | `LogEntry`, `LogLevel` — структура лог-записи             |
| `span.rs`          | `Span`, `SpanStatus`, `SpanEvent` — distributed tracing   |
| `loki.rs`          | Loki-совместимые request/response типы, `LabelMatcher`    |
| `label_selector.rs`| Парсер label selector синтаксиса `{key="val", k2!="v2"}`  |
| `otlp.rs`          | OTLP типы и конвертация в внутреннюю модель Span          |

### storage

Персистентное хранилище на RocksDB.

**RocksStore** — основная реализация. Использует две column families:

- **logs** — лог-записи. Ключ: `[8B timestamp_nanos BE][8B sequence BE]` (16 байт). Естественная хронологическая сортировка.
- **spans** — спаны. Ключ: `"{trace_id}:{span_id}"`. Позволяет prefix-итерацию по trace_id.

Значения хранятся как JSON (`serde_json`).

Все публичные методы — `async`, внутри используют `tokio::task::spawn_blocking` чтобы не блокировать async runtime.

### collector

Сервис приёма данных. Эндпоинты:

- `POST /v1/logs` — приём массива `LogEntry` (JSON)
- `POST /v1/traces` — приём массива `Span` (JSON)
- `POST /otlp/v1/traces` — приём трейсов в формате OpenTelemetry (OTLP HTTP+JSON)

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

Единый серверный бинарник (`rgrab`), объединяющий collector + web + loki_api в одном процессе. Конфигурируется через CLI аргументы (clap) и/или TOML конфиг.

Необходим потому что RocksDB блокирует директорию БД — два процесса не могут открыть одну БД одновременно.

### tui

TUI-клиент (`rgrab-tui`) — отдельный бинарник на ratatui + crossterm. Подключается к серверу по HTTP, предоставляет интерфейс для просмотра логов и трейсов в терминале.

- Два таба: Logs и Traces
- Sidebar с фильтрацией по лейблам
- Поиск по тексту
- Live tail с автообновлением
- Раскрытие трейсов с waterfall-визуализацией спанов
- Показ ошибки подключения при недоступности сервера

## Поток данных

```
                    ┌──────────────────┐
                    │   Приложения     │
                    │ (SDK, promtail)  │
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
  POST /v1/logs     POST /v1/traces    POST /otlp/v1/traces
  POST /rgrab/api/v1/push              (OpenTelemetry)
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                    ┌────────▼─────────┐
                    │   RocksStore     │
                    │  (column families│
                    │   logs / spans)  │
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
  GET /api/logs     GET /api/traces    GET /rgrab/api/v1/query*
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
         rgrab-tui      Grafana          curl
         (TUI)       (Loki source)
```

## Конфигурация

Сервер поддерживает три способа конфигурации (приоритет: CLI > TOML > defaults):

1. **CLI аргументы** — `rgrab --data-dir /path --listen 0.0.0.0:3000 --log-level debug`
2. **TOML конфиг** — `rgrab --config /etc/rgrab/rgrab.toml`
3. **Defaults** — `data_dir=./data/rgrab`, `listen=0.0.0.0:3000`, `log_level=info`

## Ключевые решения

1. **RocksDB** — встроенная БД, не требует отдельного сервера. Персистентность из коробки.
2. **Единый бинарник** — упрощает деплой, избегает проблемы блокировки БД.
3. **Loki-совместимый API** — свои пути (`/rgrab/api/v1/...`), но формат ответов идентичен Loki. Позволяет использовать Grafana как UI.
4. **OTLP поддержка** — приём трейсов в стандартном формате OpenTelemetry (HTTP+JSON).
5. **TUI вместо Web UI** — терминальный клиент, не требует браузера, работает по SSH.
6. **Label-based filtering** — логи фильтруются по лейблам, как в Loki. Поддержка `=`, `!=`, `=~`, `!~`.
7. **clap + TOML** — конфигурация через CLI и/или файл, приоритет CLI над конфигом.

## Стек технологий

| Компонент       | Технология                |
|-----------------|---------------------------|
| Язык            | Rust (edition 2024)       |
| Async runtime   | Tokio                     |
| HTTP framework  | Axum 0.8                  |
| Хранилище       | RocksDB 0.22              |
| Сериализация    | serde / serde_json        |
| CLI             | clap 4                    |
| Конфиг          | toml 0.8                  |
| TUI             | ratatui 0.29 + crossterm 0.28 |
| HTTP клиент     | reqwest 0.12              |
| Логирование     | tracing / tracing-subscriber |
| Error handling  | anyhow                    |
