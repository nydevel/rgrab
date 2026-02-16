# Использование rgrab

## Запуск сервера

```bash
# Минимальный запуск (дефолты: порт 3000, БД ./data/rgrab)
rgrab

# С CLI параметрами
rgrab --data-dir /var/lib/rgrab --listen 0.0.0.0:3000 --log-level info

# С TOML конфигом
rgrab --config /etc/rgrab/rgrab.toml

# Dev-режим через cargo
cargo run -p server -- --data-dir ./data --log-level debug
```

## TUI-клиент

TUI-клиент подключается к работающему серверу по HTTP.

```bash
# Подключение к локальному серверу
rgrab-tui

# Подключение к удалённому серверу
rgrab-tui --server http://192.168.1.100:3000

# Dev-режим через cargo
cargo run -p tui -- --server http://localhost:3000
```

### Управление TUI

| Клавиша     | Действие                          |
|-------------|-----------------------------------|
| `Tab`       | Переключение Logs / Traces        |
| `j/k`, `Up/Down` | Скролл                      |
| `h/l`, `Left/Right` | Переключение sidebar / main |
| `/`         | Режим поиска                      |
| `Enter`     | Выбрать лейбл / раскрыть трейс   |
| `Esc`       | Выход из поиска / закрыть спаны   |
| `L`         | Включить/выключить live tail      |
| `r`         | Обновить данные                   |
| `+/-`       | Увеличить/уменьшить limit         |
| `q`         | Выход                             |

При отсутствии подключения к серверу TUI показывает popup с адресом сервера и ошибкой. Переподключение происходит автоматически при включённом live tail.

---

## Подключение приложений к rgrab

### Отправка логов (JSON)

```bash
curl -X POST http://localhost:3000/v1/logs \
  -H 'Content-Type: application/json' \
  -d '[
    {
      "timestamp": "2024-01-15T10:30:00Z",
      "level": "INFO",
      "message": "User logged in",
      "labels": {
        "service": "auth",
        "env": "production",
        "host": "web-01"
      },
      "trace_id": "abc123",
      "span_id": "span456"
    },
    {
      "timestamp": "2024-01-15T10:30:01Z",
      "level": "ERROR",
      "message": "Database connection failed",
      "labels": {
        "service": "api",
        "env": "production"
      },
      "trace_id": null,
      "span_id": null
    }
  ]'
```

Уровни логирования: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`, `FATAL`.

### Отправка трейсов (JSON)

```bash
curl -X POST http://localhost:3000/v1/traces \
  -H 'Content-Type: application/json' \
  -d '[
    {
      "trace_id": "abc123",
      "span_id": "span456",
      "parent_span_id": null,
      "operation_name": "HTTP GET /users",
      "service_name": "api-gateway",
      "start_time": "2024-01-15T10:30:00Z",
      "end_time": "2024-01-15T10:30:00.250Z",
      "status": "OK",
      "attributes": {"http.method": "GET", "http.status_code": "200"},
      "events": []
    }
  ]'
```

Статусы спанов: `UNSET`, `OK`, `ERROR`.

### Отправка трейсов через OpenTelemetry (OTLP HTTP+JSON)

rgrab поддерживает приём трейсов в стандартном формате OpenTelemetry. Это позволяет подключить любое приложение, использующее OpenTelemetry SDK.

```bash
curl -X POST http://localhost:3000/otlp/v1/traces \
  -H 'Content-Type: application/json' \
  -d '{
    "resourceSpans": [
      {
        "resource": {
          "attributes": [
            {"key": "service.name", "value": {"stringValue": "my-service"}}
          ]
        },
        "scopeSpans": [
          {
            "scope": {"name": "my-lib", "version": "1.0.0"},
            "spans": [
              {
                "traceId": "0af7651916cd43dd8448eb211c80319c",
                "spanId": "b7ad6b7169203331",
                "parentSpanId": "",
                "name": "HTTP GET /api/users",
                "kind": 2,
                "startTimeUnixNano": "1705312200000000000",
                "endTimeUnixNano": "1705312200250000000",
                "attributes": [
                  {"key": "http.method", "value": {"stringValue": "GET"}},
                  {"key": "http.status_code", "value": {"intValue": "200"}}
                ],
                "events": [],
                "status": {"code": 1}
              }
            ]
          }
        ]
      }
    ]
  }'
```

#### Настройка OpenTelemetry SDK

Для подключения приложения укажите OTLP HTTP exporter endpoint:

**Python** (opentelemetry-sdk):
```python
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter

exporter = OTLPSpanExporter(endpoint="http://localhost:3000/otlp/v1/traces")
```

**Go** (go.opentelemetry.io):
```go
exporter, _ := otlptracehttp.New(ctx,
    otlptracehttp.WithEndpoint("localhost:3000"),
    otlptracehttp.WithURLPath("/otlp/v1/traces"),
    otlptracehttp.WithInsecure(),
)
```

**Node.js** (@opentelemetry/exporter-trace-otlp-http):
```javascript
const exporter = new OTLPTraceExporter({
  url: 'http://localhost:3000/otlp/v1/traces',
});
```

**Rust** (opentelemetry-otlp):
```rust
let exporter = opentelemetry_otlp::SpanExporter::builder()
    .with_http()
    .with_endpoint("http://localhost:3000/otlp/v1/traces")
    .build()?;
```

**Переменная окружения** (универсально):
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:3000
export OTEL_EXPORTER_OTLP_PROTOCOL=http/json
```

### Отправка логов в формате Loki

```bash
curl -X POST http://localhost:3000/rgrab/api/v1/push \
  -H 'Content-Type: application/json' \
  -d '{
    "streams": [
      {
        "stream": {
          "service": "auth",
          "level": "info",
          "env": "production"
        },
        "values": [
          ["1705312200000000000", "User logged in"],
          ["1705312201000000000", "Session created", {"trace_id": "abc123"}]
        ]
      }
    ]
  }'
```

Формат values: `["timestamp_nanoseconds", "message", {metadata}]`.
Третий элемент (metadata) опционален. Может содержать `trace_id` и `span_id`.

---

## Запросы к API

### Native API

```bash
# Последние 100 логов
curl http://localhost:3000/api/logs

# Последние 10 логов
curl http://localhost:3000/api/logs?limit=10

# Все спаны конкретного трейса
curl http://localhost:3000/api/traces?trace_id=abc123

# Последние 50 спанов (все трейсы)
curl http://localhost:3000/api/traces?limit=50
```

### Loki-совместимый API

#### Запрос логов

```bash
# Все логи сервиса auth
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="auth"}'

# Логи уровня error, последние 10
curl 'http://localhost:3000/rgrab/api/v1/query?query={level="error"}&limit=10'

# С направлением (старые первыми)
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="api"}&direction=forward'
```

Параметры:
| Параметр    | Обязателен | Описание                              | По умолчанию |
|-------------|------------|---------------------------------------|--------------|
| `query`     | да         | Label selector: `{key="val"}`         | -            |
| `limit`     | нет        | Макс. количество записей              | 100          |
| `time`      | нет        | Timestamp в наносекундах (конец)      | сейчас       |
| `direction` | нет        | `forward` или `backward`              | backward     |

#### Запрос за диапазон времени

```bash
curl 'http://localhost:3000/rgrab/api/v1/query_range?query={service="api"}&start=1705312200000000000&end=1705315800000000000'
```

#### Лейблы

```bash
# Список всех имён лейблов
curl http://localhost:3000/rgrab/api/v1/labels

# Все значения конкретного лейбла
curl http://localhost:3000/rgrab/api/v1/label/service/values
```

---

## Синтаксис label selector

| Оператор | Значение           | Пример                    |
|----------|--------------------|---------------------------|
| `=`      | Точное совпадение  | `{service="auth"}`        |
| `!=`     | Не равно           | `{level!="debug"}`        |
| `=~`     | Regex совпадение   | `{service=~"api.*"}`      |
| `!~`     | Regex не совпадает | `{env!~"dev\|staging"}`   |

Примеры:
```
{service="auth"}
{service="auth", level="error"}
{service=~"(auth|api)", env="production"}
{service="auth", level!="trace", level!="debug"}
{}
```

---

## Интеграция с Grafana

1. В Grafana добавить Data Source типа **Loki**
2. URL: `http://<rgrab-host>:3000/rgrab`
3. Сохранить и проверить подключение
4. В Explore выбрать datasource и писать запросы в формате label selector

---

## Примеры

### Мониторинг ошибок

```bash
# Все ошибки за последний час
curl 'http://localhost:3000/rgrab/api/v1/query_range?query={level="error"}&start='$(date -d '1 hour ago' +%s)000000000'&end='$(date +%s)000000000

# Ошибки конкретного сервиса
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="api",level=~"error|fatal"}&limit=50'
```

### Отслеживание трейса

```bash
# Все спаны трейса
curl http://localhost:3000/api/traces?trace_id=abc123

# Логи по trace_id
curl http://localhost:3000/api/logs?limit=1000 | jq '.[] | select(.trace_id == "abc123")'
```

### Статистика по лейблам

```bash
# Какие сервисы отправляют логи
curl http://localhost:3000/rgrab/api/v1/label/service/values

# Какие окружения есть
curl http://localhost:3000/rgrab/api/v1/label/env/values
```
