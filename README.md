# OAuth 2.0 Authorization Server

Полнофункциональный OAuth 2.0 сервер авторизации, написанный на Rust с использованием Actix-web и PostgreSQL.

## Возможности

-  **OAuth 2.0 Authorization Code Flow** с поддержкой PKCE
-  **OAuth 2.0 Client Credentials Flow**
-  **Refresh Token Flow**
-  JWT Access Tokens с RS256/HS256
-  Управление OAuth клиентами
-  Scope-based авторизация
-  Session-based аутентификация для пользователей
-  Consent Screen для авторизации приложений
-  Token Revocation
-  Защищенные API endpoints с middleware

## Технологический стек

- **Язык**: Rust
- **Web Framework**: Actix-web 4.4
- **База данных**: PostgreSQL
- **ORM**: SQLx
- **Токены**: JSON Web Tokens (jsonwebtoken)
- **Хеширование**: bcrypt
- **Сессии**: actix-session

## Требования

- Rust 1.70+
- PostgreSQL 13+
- Cargo

## Установка и запуск

### 1. Клонирование репозитория

```bash
git clone <repository-url>
cd AuthService
```

### 2. Настройка базы данных

Создайте базу данных PostgreSQL:

```sql
CREATE DATABASE oauth_service;
```

### 3. Настройка переменных окружения

Создайте файл `.env` в корне проекта:

```env
DATABASE_URL=postgresql://postgres:password@localhost:5432/oauth_service
HOST=127.0.0.1
PORT=8080
JWT_SECRET=your-super-secret-jwt-key-change-in-production
SESSION_KEY=your-session-key-must-be-at-least-64-bytes-long-change-this-in-prod
```

### 4. Запуск сервера

```bash
cargo run
```

Сервер запустится на `http://127.0.0.1:8080`

## API Документация

### Эндпоинты регистрации и аутентификации

#### Регистрация пользователя

```http
POST /api/register
Content-Type: application/json

{
  "username": "john_doe",
  "email": "john@example.com",
  "password": "securepassword123"
}
```

**Ответ:**
```json
{
  "id": "uuid",
  "username": "john_doe",
  "email": "john@example.com",
  "created_at": "2024-01-01T00:00:00Z"
}
```

#### Вход в систему

```http
GET /auth/login
```

Отображает HTML форму входа.

```http
POST /auth/login
Content-Type: application/json

{
  "email": "john@example.com",
  "password": "securepassword123"
}
```

#### Выход из системы

```http
POST /auth/logout
```

#### Получение информации о текущем пользователе

```http
GET /auth/me
```

### OAuth 2.0 Эндпоинты

#### Регистрация OAuth клиента

```http
POST /oauth/clients
Content-Type: application/json

{
  "client_name": "My Application",
  "redirect_uris": ["https://myapp.com/callback"],
  "allowed_scopes": ["read:profile", "write:profile", "read:email"],
  "grant_types": ["authorization_code", "refresh_token"],
  "is_confidential": true
}
```

**Ответ:**
```json
{
  "client_id": "client_xyz123...",
  "client_secret": "secret_abc456...",
  "client_name": "My Application",
  "redirect_uris": ["https://myapp.com/callback"],
  "allowed_scopes": ["read:profile", "write:profile", "read:email"],
  "grant_types": ["authorization_code", "refresh_token"]
}
```

**Важно**: Сохраните `client_secret`, он показывается только один раз!

#### Authorization Code Flow

**Шаг 1**: Перенаправьте пользователя на authorization endpoint:

```http
GET /oauth/authorize?response_type=code&client_id=CLIENT_ID&redirect_uri=REDIRECT_URI&scope=read:profile&state=RANDOM_STATE&code_challenge=CHALLENGE&code_challenge_method=S256
```

Параметры:
- `response_type`: `code` (обязательный)
- `client_id`: ID вашего клиента (обязательный)
- `redirect_uri`: URI для перенаправления (обязательный)
- `scope`: Запрашиваемые разрешения (опционально)
- `state`: Случайная строка для защиты от CSRF (рекомендуется)
- `code_challenge`: PKCE challenge (опционально, но рекомендуется)
- `code_challenge_method`: `S256` или `plain` (опционально)

Пользователь увидит consent screen и после одобрения будет перенаправлен:

```
https://myapp.com/callback?code=AUTHORIZATION_CODE&state=RANDOM_STATE
```

**Шаг 2**: Обмен authorization code на токены:

```http
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&code=AUTHORIZATION_CODE&redirect_uri=REDIRECT_URI&client_id=CLIENT_ID&client_secret=CLIENT_SECRET&code_verifier=VERIFIER
```

**Ответ:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "refresh_token_string",
  "scope": "read:profile"
}
```

#### Client Credentials Flow

```http
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=client_credentials&client_id=CLIENT_ID&client_secret=CLIENT_SECRET&scope=read:profile
```

#### Refresh Token Flow

```http
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token&refresh_token=REFRESH_TOKEN&client_id=CLIENT_ID&client_secret=CLIENT_SECRET
```

#### Token Revocation

```http
POST /oauth/revoke
Content-Type: application/json

{
  "token": "access_or_refresh_token"
}
```

### Защищенные эндпоинты

Все эндпоинты в `/api/protected/*` требуют Bearer токен в заголовке:

```http
Authorization: Bearer YOUR_ACCESS_TOKEN
```

#### Получение профиля

```http
GET /api/protected/profile
Authorization: Bearer YOUR_ACCESS_TOKEN
```

**Ответ:**
```json
{
  "message": "This is a protected resource",
  "user_id": "uuid",
  "client_id": "client_id",
  "scopes": "read:profile write:profile"
}
```

#### Получение данных (требует scope `read:profile`)

```http
GET /api/protected/data
Authorization: Bearer YOUR_ACCESS_TOKEN
```

## Scopes (Области доступа)

По умолчанию доступны следующие scopes:

- `read:profile` - Чтение профиля пользователя
- `write:profile` - Изменение профиля пользователя
- `read:email` - Чтение email адреса
- `admin` - Административный доступ

Вы можете добавить свои scopes в таблицу `oauth_scopes`.

## Структура базы данных

### Таблицы:

1. **users** - Пользователи системы
2. **oauth_clients** - Зарегистрированные OAuth клиенты
3. **oauth_authorization_codes** - Временные authorization codes
4. **oauth_tokens** - Access и refresh токены
5. **oauth_scopes** - Доступные области доступа

## Безопасность

### Рекомендации для production:

1.  Используйте HTTPS (установите `cookie_secure(true)` в SessionMiddleware)
2.  Измените `JWT_SECRET` на криптографически стойкий ключ
3.  Измените `SESSION_KEY` на случайную строку длиной 64+ байта
4.  Используйте secure password для PostgreSQL
5.  Настройте CORS политики
6.  Добавьте rate limiting для `/oauth/token`
7.  Регулярно очищайте истекшие токены
8.  Используйте PKCE для public clients (мобильные/SPA приложения)

## PKCE (Proof Key for Code Exchange)

PKCE защищает Authorization Code Flow от атак перехвата кода.

### Генерация PKCE параметров:

```javascript
// Генерация code_verifier
const code_verifier = generateRandomString(128);

// Генерация code_challenge
const code_challenge = base64UrlEncode(sha256(code_verifier));
```

### Использование:

1. При запросе authorization code передайте `code_challenge` и `code_challenge_method=S256`
2. При обмене кода на токены передайте `code_verifier`

## Примеры использования

### Пример клиента на JavaScript

```javascript
// 1. Регистрация клиента
const registerClient = async () => {
  const response = await fetch('http://localhost:8080/oauth/clients', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_name: 'My App',
      redirect_uris: ['http://localhost:3000/callback'],
      allowed_scopes: ['read:profile', 'read:email'],
      grant_types: ['authorization_code', 'refresh_token'],
      is_confidential: true
    })
  });
  return await response.json();
};

// 2. Перенаправление на authorization
const authorize = (clientId, redirectUri) => {
  const state = generateRandomString(32);
  sessionStorage.setItem('oauth_state', state);
  
  const params = new URLSearchParams({
    response_type: 'code',
    client_id: clientId,
    redirect_uri: redirectUri,
    scope: 'read:profile read:email',
    state: state
  });
  
  window.location.href = `http://localhost:8080/oauth/authorize?${params}`;
};

// 3. Обработка callback и получение токенов
const handleCallback = async (code, clientId, clientSecret, redirectUri) => {
  const response = await fetch('http://localhost:8080/oauth/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      grant_type: 'authorization_code',
      code: code,
      redirect_uri: redirectUri,
      client_id: clientId,
      client_secret: clientSecret
    })
  });
  return await response.json();
};

// 4. Использование access token
const getProtectedResource = async (accessToken) => {
  const response = await fetch('http://localhost:8080/api/protected/profile', {
    headers: { 'Authorization': `Bearer ${accessToken}` }
  });
  return await response.json();
};
```

## Разработка

### Запуск тестов

```bash
cargo test
```

### Форматирование кода

```bash
cargo fmt
```

### Проверка кода

```bash
cargo clippy
```

## Архитектура

```
src/
├── main.rs                  # Точка входа, конфигурация сервера
├── lib.rs                   # Экспорт модулей
├── models.rs                # Модели данных и DTO
├── database.rs              # Подключение к БД и миграции
├── services.rs              # Бизнес-логика пользователей
├── handlers.rs              # HTTP handlers для API
├── token_service.rs         # Генерация и валидация JWT токенов
├── client_service.rs        # Управление OAuth клиентами
├── oauth_service.rs         # OAuth 2.0 flows логика
├── auth_handlers.rs         # Handlers для аутентификации
├── oauth_handlers.rs        # Handlers для OAuth endpoints
├── protected_handlers.rs    # Защищенные endpoints
└── middleware.rs            # Auth и scope validation middleware
```

## Лицензия

[LICENSE](LICENSE.md)
