# Packaging и APT-репозиторий

## Сборка .deb пакета

### Требования

```bash
cargo install cargo-deb
```

### Сборка

```bash
# Полная сборка (release + .deb)
cargo deb -p server

# Или поэтапно
cargo build --release -p server
cargo deb -p server --no-build
```

Результат: `target/debian/rgrab_<version>-1_amd64.deb`

### Что внутри пакета

```
/usr/bin/rgrab                    # бинарник
/usr/share/rgrab/static/          # веб-интерфейс
/etc/rgrab/rgrab.conf             # конфигурация (conffile, сохраняется при обновлении)
/usr/lib/systemd/system/rgrab.service  # systemd unit
```

При установке автоматически:
- Создаётся пользователь `rgrab` (системный, без shell)
- Создаётся директория `/var/lib/rgrab` для данных RocksDB
- Регистрируется systemd service (не запускается автоматически)

## Установка из .deb файла

```bash
sudo dpkg -i rgrab_0.1.0-1_amd64.deb
sudo apt-get install -f  # доустановить зависимости если нужно

sudo systemctl enable rgrab
sudo systemctl start rgrab
```

## Настройка APT-репозитория

### Вариант 1: GitHub Releases (простейший)

1. Загрузить `.deb` в GitHub Release
2. Пользователи скачивают и ставят напрямую:

```bash
wget https://github.com/nydevel/rgrab/releases/download/v0.1.0/rgrab_0.1.0-1_amd64.deb
sudo dpkg -i rgrab_0.1.0-1_amd64.deb
```

### Вариант 2: Собственный APT-репозиторий

#### Генерация репозитория

```bash
# Собрать .deb
cargo deb -p server

# Сгенерировать репо
./packaging/build-repo.sh ./apt-repo
```

#### Подпись GPG (рекомендуется)

```bash
# Создать GPG ключ (один раз)
gpg --full-generate-key
# Имя: rgrab, Email: your@email.com

# Экспортировать публичный ключ
gpg --armor --export rgrab > apt-repo/rgrab.gpg.key

# Пересобрать репо (теперь подпишет)
./packaging/build-repo.sh ./apt-repo
```

#### Хостинг

Содержимое `apt-repo/` разместить на любом HTTP-сервере:
- **GitHub Pages** — бесплатно, просто
- **nginx** — `location /apt { root /var/www; autoindex on; }`
- **S3/MinIO** — для масштабных деплоев

Структура на сервере:
```
https://repo.example.com/
├── pool/
│   └── rgrab_0.1.0-1_amd64.deb
├── Packages
├── Packages.gz
├── Release
├── Release.gpg      (если подписан)
├── InRelease         (если подписан)
└── rgrab.gpg.key     (публичный ключ)
```

#### Подключение на клиенте

```bash
# Добавить GPG ключ
curl -fsSL https://repo.example.com/rgrab.gpg.key | sudo gpg --dearmor -o /usr/share/keyrings/rgrab.gpg

# Добавить репозиторий
echo "deb [signed-by=/usr/share/keyrings/rgrab.gpg] https://repo.example.com/ ./" | \
  sudo tee /etc/apt/sources.list.d/rgrab.list

# Установить
sudo apt-get update
sudo apt-get install rgrab
```

Без GPG подписи (не рекомендуется для продакшна):
```bash
echo "deb [trusted=yes] https://repo.example.com/ ./" | \
  sudo tee /etc/apt/sources.list.d/rgrab.list
sudo apt-get update
sudo apt-get install rgrab
```

## Управление сервисом

```bash
# Запуск / остановка
sudo systemctl start rgrab
sudo systemctl stop rgrab

# Автозапуск
sudo systemctl enable rgrab

# Логи
sudo journalctl -u rgrab -f

# Статус
sudo systemctl status rgrab
```

## Конфигурация

Файл `/etc/rgrab/rgrab.conf`:

```
RGRAB_DATA_DIR=/var/lib/rgrab
RGRAB_STATIC_DIR=/usr/share/rgrab/static
RGRAB_LISTEN=0.0.0.0:3000
RUST_LOG=info
```

После изменения:
```bash
sudo systemctl restart rgrab
```

## Обновление

```bash
sudo apt-get update
sudo apt-get upgrade rgrab
# или
sudo dpkg -i rgrab_<new_version>-1_amd64.deb
```

Конфиг `/etc/rgrab/rgrab.conf` сохраняется при обновлении (conffile).
Данные в `/var/lib/rgrab` не затрагиваются.

## Удаление

```bash
# Удалить пакет (данные сохранятся)
sudo apt-get remove rgrab

# Полное удаление (включая конфиг)
sudo apt-get purge rgrab

# Удалить данные вручную
sudo rm -rf /var/lib/rgrab
```

## Cross-compilation

Для сборки под другую архитектуру:

```bash
# Установить target
rustup target add aarch64-unknown-linux-gnu

# Собрать (нужен линкер для целевой платформы)
cargo deb -p server --target aarch64-unknown-linux-gnu
```
