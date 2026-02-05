# Примеры использования OAuth 2.0 API

Этот файл содержит практические примеры использования OAuth 2.0 сервиса.

## Подготовка

Убедитесь, что сервер запущен:

```bash
cargo run
```

Сервер будет доступен по адресу: `http://127.0.0.1:8080`

## 1. Регистрация пользователя

```bash
curl -X POST http://127.0.0.1:8080/api/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "john_doe",
    "email": "john@example.com",
    "password": "securepassword123"
  }'
```

**Ответ:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "john_doe",
  "email": "john@example.com",
  "created_at": "2024-01-01T12:00:00Z"
}
```

## 2. Вход в систему

```bash
curl -X POST http://127.0.0.1:8080/auth/login \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{
    "email": "john@example.com",
    "password": "securepassword123"
  }'
```

Флаг `-c cookies.txt` сохраняет cookie сессии для последующих запросов.

## 3. Регистрация OAuth клиента

```bash
curl -X POST http://127.0.0.1:8080/oauth/clients \
  -H "Content-Type: application/json" \
  -d '{
    "client_name": "My Mobile App",
    "redirect_uris": ["http://localhost:3000/callback"],
    "allowed_scopes": ["read:profile", "write:profile", "read:email"],
    "grant_types": ["authorization_code", "refresh_token"],
    "is_confidential": true
  }'
```

**Ответ:**
```json
{
  "client_id": "client_abc123xyz...",
  "client_secret": "secret_def456uvw...",
  "client_name": "My Mobile App",
  "redirect_uris": ["http://localhost:3000/callback"],
  "allowed_scopes": ["read:profile", "write:profile", "read:email"],
  "grant_types": ["authorization_code", "refresh_token"]
}
```

⚠️ **Важно:** Сохраните `client_secret` - он показывается только один раз!

## 4. Authorization Code Flow (полный цикл)

### Шаг 1: Получение authorization code

Откройте в браузере (замените CLIENT_ID на ваш):

```
http://127.0.0.1:8080/oauth/authorize?response_type=code&client_id=CLIENT_ID&redirect_uri=http://localhost:3000/callback&scope=read:profile%20read:email&state=random_state_123
```

1. Если не авторизованы - будете перенаправлены на страницу входа
2. После входа увидите consent screen
3. При одобрении будете перенаправлены на: 
   ```
   http://localhost:3000/callback?code=AUTHORIZATION_CODE&state=random_state_123
   ```

### Шаг 2: Обмен кода на токены

```bash
curl -X POST http://127.0.0.1:8080/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=AUTHORIZATION_CODE" \
  -d "redirect_uri=http://localhost:3000/callback" \
  -d "client_id=CLIENT_ID" \
  -d "client_secret=CLIENT_SECRET"
```

**Ответ:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "refresh_abc123xyz...",
  "scope": "read:profile read:email"
}
```

## 5. Authorization Code Flow с PKCE

### Генерация PKCE параметров (Python)

```python
import hashlib
import base64
import secrets

# Генерация code_verifier
code_verifier = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode('utf-8').rstrip('=')

# Генерация code_challenge
code_challenge = base64.urlsafe_b64encode(
    hashlib.sha256(code_verifier.encode()).digest()
).decode('utf-8').rstrip('=')

print(f"Code Verifier: {code_verifier}")
print(f"Code Challenge: {code_challenge}")
```

### Шаг 1: Authorization с PKCE

```
http://127.0.0.1:8080/oauth/authorize?response_type=code&client_id=CLIENT_ID&redirect_uri=http://localhost:3000/callback&scope=read:profile&state=xyz&code_challenge=CODE_CHALLENGE&code_challenge_method=S256
```

### Шаг 2: Обмен кода с code_verifier

```bash
curl -X POST http://127.0.0.1:8080/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=AUTHORIZATION_CODE" \
  -d "redirect_uri=http://localhost:3000/callback" \
  -d "client_id=CLIENT_ID" \
  -d "client_secret=CLIENT_SECRET" \
  -d "code_verifier=CODE_VERIFIER"
```

## 6. Client Credentials Flow

```bash
curl -X POST http://127.0.0.1:8080/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=CLIENT_ID" \
  -d "client_secret=CLIENT_SECRET" \
  -d "scope=read:profile"
```

**Ответ:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "scope": "read:profile"
}
```

## 7. Обновление токена (Refresh Token Flow)

```bash
curl -X POST http://127.0.0.1:8080/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token" \
  -d "refresh_token=REFRESH_TOKEN" \
  -d "client_id=CLIENT_ID" \
  -d "client_secret=CLIENT_SECRET"
```

## 8. Использование Access Token

### Защищенный endpoint: Profile

```bash
curl -X GET http://127.0.0.1:8080/api/protected/profile \
  -H "Authorization: Bearer ACCESS_TOKEN"
```

**Ответ:**
```json
{
  "message": "This is a protected resource",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "client_id": "client_abc123xyz",
  "scopes": "read:profile read:email"
}
```

### Защищенный endpoint с scope check: Data

```bash
curl -X GET http://127.0.0.1:8080/api/protected/data \
  -H "Authorization: Bearer ACCESS_TOKEN"
```

**Успешный ответ (с scope read:profile):**
```json
{
  "message": "This is sensitive data",
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "permissions": ["read:profile", "read:email"]
  }
}
```

**Ошибка (без нужного scope):**
```json
{
  "error": "Insufficient permissions. Required scope: read:profile"
}
```

## 9. Отзыв токена

```bash
curl -X POST http://127.0.0.1:8080/oauth/revoke \
  -H "Content-Type: application/json" \
  -d '{
    "token": "ACCESS_OR_REFRESH_TOKEN"
  }'
```

## 10. Проверка информации о текущем пользователе

```bash
curl -X GET http://127.0.0.1:8080/auth/me \
  -b cookies.txt
```

## Пример полного workflow на JavaScript

```javascript
// 1. Регистрация пользователя
async function registerUser() {
  const response = await fetch('http://127.0.0.1:8080/api/register', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      username: 'testuser',
      email: 'test@example.com',
      password: 'password123'
    })
  });
  return await response.json();
}

// 2. Вход
async function login() {
  const response = await fetch('http://127.0.0.1:8080/auth/login', {
    method: 'POST',
    credentials: 'include', // Important for cookies
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      email: 'test@example.com',
      password: 'password123'
    })
  });
  return await response.json();
}

// 3. Регистрация OAuth клиента
async function registerClient() {
  const response = await fetch('http://127.0.0.1:8080/oauth/clients', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_name: 'My SPA',
      redirect_uris: ['http://localhost:3000/callback'],
      allowed_scopes: ['read:profile', 'read:email'],
      grant_types: ['authorization_code', 'refresh_token'],
      is_confidential: false // Public client для SPA
    })
  });
  return await response.json();
}

// 4. Инициация Authorization Flow
function startOAuthFlow(clientId) {
  const state = crypto.randomUUID();
  sessionStorage.setItem('oauth_state', state);
  
  const params = new URLSearchParams({
    response_type: 'code',
    client_id: clientId,
    redirect_uri: 'http://localhost:3000/callback',
    scope: 'read:profile read:email',
    state: state
  });
  
  window.location.href = `http://127.0.0.1:8080/oauth/authorize?${params}`;
}

// 5. Обработка callback
async function handleCallback() {
  const params = new URLSearchParams(window.location.search);
  const code = params.get('code');
  const state = params.get('state');
  
  // Проверка state
  if (state !== sessionStorage.getItem('oauth_state')) {
    throw new Error('Invalid state parameter');
  }
  
  // Обмен кода на токены
  const response = await fetch('http://127.0.0.1:8080/oauth/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      grant_type: 'authorization_code',
      code: code,
      redirect_uri: 'http://localhost:3000/callback',
      client_id: 'YOUR_CLIENT_ID',
      client_secret: 'YOUR_CLIENT_SECRET' // Не используйте в браузере для production!
    })
  });
  
  const tokens = await response.json();
  sessionStorage.setItem('access_token', tokens.access_token);
  sessionStorage.setItem('refresh_token', tokens.refresh_token);
  
  return tokens;
}

// 6. Использование access token
async function getProtectedData() {
  const accessToken = sessionStorage.getItem('access_token');
  
  const response = await fetch('http://127.0.0.1:8080/api/protected/profile', {
    headers: {
      'Authorization': `Bearer ${accessToken}`
    }
  });
  
  if (response.status === 401) {
    // Токен истек, попробуйте refresh
    await refreshAccessToken();
    return getProtectedData(); // Retry
  }
  
  return await response.json();
}

// 7. Обновление токена
async function refreshAccessToken() {
  const refreshToken = sessionStorage.getItem('refresh_token');
  
  const response = await fetch('http://127.0.0.1:8080/oauth/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      grant_type: 'refresh_token',
      refresh_token: refreshToken,
      client_id: 'YOUR_CLIENT_ID',
      client_secret: 'YOUR_CLIENT_SECRET'
    })
  });
  
  const tokens = await response.json();
  sessionStorage.setItem('access_token', tokens.access_token);
  sessionStorage.setItem('refresh_token', tokens.refresh_token);
  
  return tokens;
}

// Полный workflow
async function fullWorkflow() {
  // 1. Регистрация
  await registerUser();
  console.log('User registered');
  
  // 2. Вход
  await login();
  console.log('User logged in');
  
  // 3. Регистрация клиента
  const client = await registerClient();
  console.log('Client registered:', client.client_id);
  
  // 4. OAuth flow
  startOAuthFlow(client.client_id);
  // Пользователь будет перенаправлен на consent screen
  // После callback обработайте код с помощью handleCallback()
}
```

## Обработка ошибок

### Неверные credentials
```json
{
  "error": "invalid_client"
}
```

### Истекший authorization code
```json
{
  "error": "Code expired"
}
```

### Недостаточно прав (scopes)
```json
{
  "error": "Missing required scope: admin"
}
```

### Отозванный токен
```json
{
  "error": "Token has been revoked"
}
```

## Тестирование с Postman

1. Импортируйте коллекцию endpoints
2. Создайте environment переменные:
   - `BASE_URL`: `http://127.0.0.1:8080`
   - `CLIENT_ID`: (после регистрации клиента)
   - `CLIENT_SECRET`: (после регистрации клиента)
   - `ACCESS_TOKEN`: (после получения токена)

3. Используйте Pre-request Scripts для автоматической установки токенов

## Best Practices

1. **Всегда используйте HTTPS в production**
2. **Не храните client_secret в коде frontend приложений**
3. **Используйте PKCE для public clients (SPA, Mobile)**
4. **Храните токены безопасно** (не в localStorage, используйте httpOnly cookies где возможно)
5. **Регулярно обновляйте access tokens с помощью refresh tokens**
6. **Реализуйте logout с отзывом токенов**
7. **Валидируйте state parameter для защиты от CSRF**
8. **Используйте минимально необходимые scopes**
