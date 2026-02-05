# Руководство по развертыванию OAuth 2.0 сервиса

## Локальная разработка

### 1. Установка зависимостей

```bash
# Установка Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Установка PostgreSQL (macOS)
brew install postgresql@14
brew services start postgresql@14

# Установка PostgreSQL (Ubuntu/Debian)
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

### 2. Настройка базы данных

```bash
# Подключение к PostgreSQL
psql postgres

# Создание базы данных и пользователя
CREATE DATABASE oauth_service;
CREATE USER oauth_user WITH ENCRYPTED PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE oauth_service TO oauth_user;
\q
```

### 3. Конфигурация проекта

Создайте файл `.env`:

```env
DATABASE_URL=postgresql://oauth_user:your_secure_password@localhost:5432/oauth_service
HOST=127.0.0.1
PORT=8080
JWT_SECRET=generate_random_32_char_string_here
SESSION_KEY=generate_random_64_char_string_here_for_session_encryption
```

### 4. Запуск сервера

```bash
# Разработка (с hot reload)
cargo watch -x run

# Обычный запуск
cargo run

# Production build
cargo build --release
./target/release/auth_service
```

## Production развертывание

### Использование Docker

#### Dockerfile

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/auth_service /app/auth_service

ENV RUST_LOG=info
EXPOSE 8080

CMD ["/app/auth_service"]
```

#### docker-compose.yml

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:14-alpine
    environment:
      POSTGRES_DB: oauth_service
      POSTGRES_USER: oauth_user
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U oauth_user"]
      interval: 10s
      timeout: 5s
      retries: 5

  oauth_server:
    build: .
    environment:
      DATABASE_URL: postgresql://oauth_user:${DB_PASSWORD}@postgres:5432/oauth_service
      HOST: 0.0.0.0
      PORT: 8080
      JWT_SECRET: ${JWT_SECRET}
      SESSION_KEY: ${SESSION_KEY}
      RUST_LOG: info
    ports:
      - "8080:8080"
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped

volumes:
  postgres_data:
```

#### .env для Docker

```env
DB_PASSWORD=your_secure_db_password
JWT_SECRET=your_jwt_secret_key_min_32_chars
SESSION_KEY=your_session_key_min_64_chars
```

### Запуск с Docker

```bash
# Сборка и запуск
docker-compose up -d

# Просмотр логов
docker-compose logs -f oauth_server

# Остановка
docker-compose down

# Остановка с удалением данных
docker-compose down -v
```

## Развертывание на VPS (Ubuntu/Debian)

### 1. Подготовка сервера

```bash
# Обновление системы
sudo apt update && sudo apt upgrade -y

# Установка зависимостей
sudo apt install -y build-essential postgresql postgresql-contrib nginx certbot python3-certbot-nginx

# Установка Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Настройка PostgreSQL

```bash
sudo -u postgres psql

CREATE DATABASE oauth_service;
CREATE USER oauth_user WITH ENCRYPTED PASSWORD 'secure_password';
GRANT ALL PRIVILEGES ON DATABASE oauth_service TO oauth_user;
\q
```

### 3. Клонирование и сборка проекта

```bash
cd /opt
sudo git clone <your-repo-url> oauth-service
cd oauth-service
sudo chown -R $USER:$USER /opt/oauth-service

# Создание .env файла
sudo nano .env
# Добавьте переменные окружения

# Сборка
cargo build --release
```

### 4. Создание systemd service

```bash
sudo nano /etc/systemd/system/oauth-service.service
```

```ini
[Unit]
Description=OAuth 2.0 Authorization Server
After=network.target postgresql.service

[Service]
Type=simple
User=www-data
Group=www-data
WorkingDirectory=/opt/oauth-service
Environment="RUST_LOG=info"
EnvironmentFile=/opt/oauth-service/.env
ExecStart=/opt/oauth-service/target/release/auth_service
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
# Запуск сервиса
sudo systemctl daemon-reload
sudo systemctl enable oauth-service
sudo systemctl start oauth-service
sudo systemctl status oauth-service
```

### 5. Настройка Nginx как reverse proxy

```bash
sudo nano /etc/nginx/sites-available/oauth-service
```

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

```bash
# Активация конфигурации
sudo ln -s /etc/nginx/sites-available/oauth-service /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl restart nginx
```

### 6. Настройка SSL с Let's Encrypt

```bash
sudo certbot --nginx -d your-domain.com
```

Certbot автоматически настроит HTTPS и обновит конфигурацию Nginx.

### 7. Настройка firewall

```bash
sudo ufw allow 'Nginx Full'
sudo ufw allow ssh
sudo ufw enable
```

## Мониторинг и логи

### Просмотр логов

```bash
# Systemd logs
sudo journalctl -u oauth-service -f

# Docker logs
docker-compose logs -f

# Nginx logs
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log
```

### Мониторинг с Prometheus (опционально)

Добавьте метрики endpoint в код и настройте Prometheus для сбора метрик.

## Резервное копирование

### Автоматическое резервное копирование PostgreSQL

```bash
# Создание скрипта backup
sudo nano /usr/local/bin/backup-oauth-db.sh
```

```bash
#!/bin/bash
BACKUP_DIR="/var/backups/oauth-db"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/oauth_backup_$TIMESTAMP.sql"

mkdir -p $BACKUP_DIR

sudo -u postgres pg_dump oauth_service > $BACKUP_FILE
gzip $BACKUP_FILE

# Удаление старых бэкапов (старше 30 дней)
find $BACKUP_DIR -name "*.sql.gz" -mtime +30 -delete

echo "Backup completed: $BACKUP_FILE.gz"
```

```bash
sudo chmod +x /usr/local/bin/backup-oauth-db.sh

# Добавление в crontab (ежедневно в 2 AM)
sudo crontab -e
0 2 * * * /usr/local/bin/backup-oauth-db.sh
```

### Восстановление из резервной копии

```bash
gunzip oauth_backup_20240101_020000.sql.gz
sudo -u postgres psql oauth_service < oauth_backup_20240101_020000.sql
```

## Обновление сервиса

```bash
cd /opt/oauth-service
git pull origin main
cargo build --release
sudo systemctl restart oauth-service
sudo systemctl status oauth-service
```

## Настройки безопасности

### 1. Ограничение доступа к базе данных

```bash
sudo nano /etc/postgresql/14/main/pg_hba.conf
```

Измените строку:
```
host    oauth_service    oauth_user    127.0.0.1/32    md5
```

### 2. Настройка rate limiting в Nginx

```nginx
http {
    limit_req_zone $binary_remote_addr zone=oauth_limit:10m rate=10r/s;
    
    server {
        location /oauth/token {
            limit_req zone=oauth_limit burst=5 nodelay;
            # ... остальная конфигурация
        }
    }
}
```

### 3. Fail2ban для защиты от brute force

```bash
sudo apt install fail2ban

sudo nano /etc/fail2ban/jail.local
```

```ini
[oauth-auth]
enabled = true
port = http,https
filter = oauth-auth
logpath = /var/log/nginx/access.log
maxretry = 5
bantime = 3600
```

## Масштабирование

### Горизонтальное масштабирование

1. Используйте load balancer (nginx, HAProxy)
2. Настройте Redis для shared session storage
3. Используйте connection pooling для PostgreSQL
4. Разделите read и write операции (PostgreSQL replication)

### Вертикальное масштабирование

1. Увеличьте количество worker threads в Actix
2. Оптимизируйте database queries (индексы)
3. Используйте caching (Redis) для частых запросов

## Проверка работоспособности

```bash
# Health check
curl http://localhost:8080/api/health

# Проверка SSL
curl https://your-domain.com/api/health

# Нагрузочное тестирование (с Apache Bench)
ab -n 1000 -c 10 http://localhost:8080/api/health
```

## Troubleshooting

### Сервис не запускается

```bash
# Проверка логов
sudo journalctl -u oauth-service -n 50

# Проверка конфигурации
cat /opt/oauth-service/.env

# Проверка подключения к БД
psql -U oauth_user -d oauth_service -h localhost
```

### Проблемы с производительностью

```bash
# Мониторинг процессов
htop

# Проверка PostgreSQL
sudo -u postgres psql -c "SELECT * FROM pg_stat_activity;"

# Проверка соединений
netstat -an | grep 8080
```

### Ошибки в логах

```bash
# Увеличение уровня логирования
export RUST_LOG=debug
sudo systemctl restart oauth-service
```

## Контрольный список для production

- [ ] Изменены все секретные ключи (JWT_SECRET, SESSION_KEY)
- [ ] Настроен HTTPS (SSL сертификат)
- [ ] Включен cookie_secure для сессий
- [ ] Настроен firewall
- [ ] Настроено автоматическое резервное копирование БД
- [ ] Настроен мониторинг и алертинг
- [ ] Настроен rate limiting
- [ ] Проверены все environment переменные
- [ ] Настроен логrotation
- [ ] Документация обновлена
- [ ] Проведено нагрузочное тестирование
- [ ] Настроен процесс обновления

## Полезные команды

```bash
# Перезапуск всех сервисов
sudo systemctl restart oauth-service nginx postgresql

# Просмотр всех портов
sudo netstat -tulpn

# Проверка использования диска
df -h
du -sh /var/lib/postgresql/14/main

# Очистка старых логов
sudo journalctl --vacuum-time=7d
```
