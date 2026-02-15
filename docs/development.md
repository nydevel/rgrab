# Разработка rgrab

## Требования

- **Rust** >= 1.85 (edition 2024)
- **libclang** — необходим для сборки `rocksdb` crate (bindgen)
- **Linux** (основная платформа)

### Установка зависимостей

Ubuntu/Debian:
```bash
sudo apt-get install -y libclang-dev
```

macOS:
```bash
brew install llvm
```

Если `libclang` установлен не в стандартном пути:
```bash
export LIBCLANG_PATH=/path/to/llvm/lib
```

## Сборка

```bash
# Проверка компиляции (быстро, без линковки)
cargo check --workspace

# Полная сборка
cargo build --workspace

# Release-сборка
cargo build --release -p server
```

## Запуск в dev-режиме

### Рекомендуемый способ: единый сервер

```bash
# Запуск с настройками по умолчанию (порт 3000, БД в ./data/rgrab)
cargo run -p server

# С кастомными параметрами
RGRAB_DATA_DIR=./my-data RGRAB_LISTEN=0.0.0.0:8080 cargo run -p server

# С включённым логированием
RUST_LOG=info cargo run -p server
```

После запуска доступны все эндпоинты на одном порту:
- Ingest: `POST /v1/logs`, `POST /v1/traces`
- Query: `GET /api/logs`, `GET /api/traces`
- Loki API: `/rgrab/api/v1/*`

### Альтернатива: отдельные сервисы

Можно запускать collector и web отдельно, но **с разными директориями БД** (RocksDB блокирует директорию):

```bash
# Терминал 1 — collector (порт 4317)
RGRAB_DATA_DIR=./data/collector cargo run -p collector

# Терминал 2 — web (порт 3000)
RGRAB_DATA_DIR=./data/web cargo run -p web
```

## Переменные окружения

| Переменная      | Описание                          | Значение по умолчанию |
|-----------------|-----------------------------------|-----------------------|
| `RGRAB_DATA_DIR`| Путь к директории RocksDB         | `./data/rgrab`        |
| `RGRAB_LISTEN`  | Адрес и порт (только для server)  | `0.0.0.0:3000`        |
| `RUST_LOG`      | Уровень логирования               | не задано (off)       |

Уровни `RUST_LOG`: `trace`, `debug`, `info`, `warn`, `error`. Можно задать per-crate:
```bash
RUST_LOG=server=info,storage=debug cargo run -p server
```

## Проверка кода

```bash
# Линтер
cargo clippy --workspace

# Форматирование
cargo fmt --all

# Проверка форматирования (CI)
cargo fmt --all -- --check
```

## Структура данных в RocksDB

БД создаётся автоматически при первом запуске. Директория содержит файлы RocksDB.

Две column families:
- `logs` — лог-записи (ключ: timestamp + sequence, значение: JSON)
- `spans` — спаны (ключ: `trace_id:span_id`, значение: JSON)

Для очистки БД — удалить директорию:
```bash
rm -rf ./data/rgrab
```

## Быстрая проверка работоспособности

```bash
# Запустить сервер
RUST_LOG=info cargo run -p server &

# Отправить лог
curl -s -X POST http://localhost:3000/v1/logs \
  -H 'Content-Type: application/json' \
  -d '[{
    "timestamp": "2024-01-01T12:00:00Z",
    "level": "INFO",
    "message": "Hello from rgrab",
    "labels": {"service": "test", "env": "dev"},
    "trace_id": null,
    "span_id": null
  }]'

# Прочитать логи
curl -s http://localhost:3000/api/logs | python3 -m json.tool
```
