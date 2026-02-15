# Использование rgrab

## Запуск

```bash
# Минимальный запуск
cargo run -p server --release

# Продакшн
RGRAB_DATA_DIR=/var/lib/rgrab RGRAB_LISTEN=0.0.0.0:3000 RUST_LOG=info \
  ./target/release/server
```

## API

rgrab предоставляет три группы эндпоинтов:

### 1. Ingest API — приём данных

#### POST /v1/logs

Приём лог-записей.

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

#### POST /v1/traces

Приём спанов (distributed tracing).

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

---

### 2. Native Query API — запросы

#### GET /api/logs

Получение последних логов.

```bash
# Последние 100 логов (по умолчанию)
curl http://localhost:3000/api/logs

# Последние 10 логов
curl http://localhost:3000/api/logs?limit=10
```

Ответ: массив `LogEntry` в формате JSON.

#### GET /api/traces

Получение спанов.

```bash
# Все спаны для конкретного трейса
curl http://localhost:3000/api/traces?trace_id=abc123

# Последние 50 спанов (все трейсы)
curl http://localhost:3000/api/traces?limit=50
```

Ответ: массив `Span` в формате JSON.

---

### 3. Loki-совместимый API

Формат ответов совместим с Grafana Loki. Можно подключить как Loki datasource в Grafana, указав URL `http://rgrab-host:3000/rgrab`.

#### POST /rgrab/api/v1/push

Приём логов в формате Loki.

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

Формат values: `["timestamp_nanoseconds", "сообщение", {метаданные}]`.
Третий элемент (метаданные) опционален. Может содержать `trace_id` и `span_id`.
Timestamp — строка с Unix epoch в наносекундах.

#### GET /rgrab/api/v1/query

Запрос логов с фильтрацией по лейблам (instant query).

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
| `query`     | да         | Label selector: `{key="val"}`         | —            |
| `limit`     | нет        | Макс. количество записей              | 100          |
| `time`      | нет        | Timestamp в наносекундах (конец)      | сейчас       |
| `direction` | нет        | `forward` или `backward`              | backward     |

Ответ:
```json
{
  "status": "success",
  "data": {
    "resultType": "streams",
    "result": [
      {
        "stream": {"service": "auth", "level": "info"},
        "values": [
          ["1705312200000000000", "User logged in"],
          ["1705312201000000000", "Session created"]
        ]
      }
    ],
    "stats": {}
  }
}
```

#### GET /rgrab/api/v1/query_range

Запрос логов за диапазон времени.

```bash
# Логи сервиса api за час
curl 'http://localhost:3000/rgrab/api/v1/query_range?query={service="api"}&start=1705312200000000000&end=1705315800000000000'
```

Параметры:
| Параметр    | Обязателен | Описание                              | По умолчанию |
|-------------|------------|---------------------------------------|--------------|
| `query`     | да         | Label selector                        | —            |
| `limit`     | нет        | Макс. количество записей              | 100          |
| `start`     | нет        | Начало диапазона (наносекунды)        | —            |
| `end`       | нет        | Конец диапазона (наносекунды)         | —            |
| `direction` | нет        | `forward` или `backward`              | backward     |

Формат ответа идентичен `/rgrab/api/v1/query`.

#### GET /rgrab/api/v1/labels

Список всех имён лейблов.

```bash
curl http://localhost:3000/rgrab/api/v1/labels

# За определённый период
curl 'http://localhost:3000/rgrab/api/v1/labels?start=1705312200000000000&end=1705315800000000000'
```

Ответ:
```json
{
  "status": "success",
  "data": ["env", "host", "level", "service"]
}
```

#### GET /rgrab/api/v1/label/{name}/values

Все значения конкретного лейбла.

```bash
curl http://localhost:3000/rgrab/api/v1/label/service/values
curl http://localhost:3000/rgrab/api/v1/label/level/values
```

Ответ:
```json
{
  "status": "success",
  "data": ["api", "auth", "gateway"]
}
```

---

## Синтаксис label selector

Label selector используется в параметре `query` Loki API.

### Операторы

| Оператор | Значение           | Пример                    |
|----------|--------------------|---------------------------|
| `=`      | Точное совпадение  | `{service="auth"}`        |
| `!=`     | Не равно           | `{level!="debug"}`        |
| `=~`     | Regex совпадение   | `{service=~"api.*"}`      |
| `!~`     | Regex не совпадает | `{env!~"dev\|staging"}`    |

### Примеры

```
# Один лейбл
{service="auth"}

# Несколько лейблов (AND логика)
{service="auth", level="error"}

# С regex
{service=~"(auth|api)", env="production"}

# Исключение
{service="auth", level!="trace", level!="debug"}

# Пустой селектор — все логи
{}
```

---

## Интеграция с Grafana

1. В Grafana добавить Data Source типа **Loki**
2. URL: `http://<rgrab-host>:3000/rgrab`
3. Сохранить и проверить подключение
4. В Explore выбрать datasource и писать запросы в формате label selector

---

## Примеры использования

### Мониторинг ошибок

```bash
# Все ошибки за последний час
curl 'http://localhost:3000/rgrab/api/v1/query_range?query={level="error"}&start='$(date -d '1 hour ago' +%s)000000000'&end='$(date +%s)000000000

# Ошибки конкретного сервиса
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="api",level=~"error|fatal"}&limit=50'
```

### Отслеживание трейса

```bash
# Найти логи по trace_id через native API
curl http://localhost:3000/api/logs?limit=1000 | jq '.[] | select(.trace_id == "abc123")'

# Найти все спаны трейса
curl http://localhost:3000/api/traces?trace_id=abc123
```

### Статистика по лейблам

```bash
# Какие сервисы отправляют логи
curl http://localhost:3000/rgrab/api/v1/label/service/values

# Какие окружения есть
curl http://localhost:3000/rgrab/api/v1/label/env/values
```
