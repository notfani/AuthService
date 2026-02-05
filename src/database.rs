use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;

pub async fn create_pool(database_url: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    // Создание таблицы users
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username VARCHAR(50) UNIQUE NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
        "#
    )
    .execute(pool)
    .await?;

    // Создание индексов для оптимизации поиска
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
        .execute(pool)
        .await?;

    // Создание таблицы oauth_clients
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS oauth_clients (
            id UUID PRIMARY KEY,
            client_id VARCHAR(255) UNIQUE NOT NULL,
            client_secret_hash VARCHAR(255) NOT NULL,
            client_name VARCHAR(255) NOT NULL,
            redirect_uris TEXT[] NOT NULL,
            allowed_scopes TEXT[] NOT NULL,
            grant_types TEXT[] NOT NULL,
            is_confidential BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_clients_client_id ON oauth_clients(client_id)")
        .execute(pool)
        .await?;

    // Создание таблицы oauth_authorization_codes
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS oauth_authorization_codes (
            id UUID PRIMARY KEY,
            code VARCHAR(255) UNIQUE NOT NULL,
            client_id VARCHAR(255) NOT NULL,
            user_id UUID NOT NULL,
            redirect_uri TEXT NOT NULL,
            scope TEXT NOT NULL,
            code_challenge VARCHAR(255),
            code_challenge_method VARCHAR(10),
            expires_at TIMESTAMPTZ NOT NULL,
            used BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_codes_code ON oauth_authorization_codes(code)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_codes_user_id ON oauth_authorization_codes(user_id)")
        .execute(pool)
        .await?;

    // Создание таблицы oauth_tokens
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS oauth_tokens (
            id UUID PRIMARY KEY,
            access_token VARCHAR(512) UNIQUE NOT NULL,
            refresh_token VARCHAR(512) UNIQUE,
            client_id VARCHAR(255) NOT NULL,
            user_id UUID,
            scope TEXT NOT NULL,
            token_type VARCHAR(50) NOT NULL DEFAULT 'Bearer',
            expires_at TIMESTAMPTZ NOT NULL,
            refresh_expires_at TIMESTAMPTZ,
            revoked BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_tokens_access_token ON oauth_tokens(access_token)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_tokens_refresh_token ON oauth_tokens(refresh_token)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_tokens_user_id ON oauth_tokens(user_id)")
        .execute(pool)
        .await?;

    // Создание таблицы oauth_scopes
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS oauth_scopes (
            id UUID PRIMARY KEY,
            scope_name VARCHAR(100) UNIQUE NOT NULL,
            description TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_scopes_name ON oauth_scopes(scope_name)")
        .execute(pool)
        .await?;

    // Добавление базовых scopes
    sqlx::query(
        r#"
        INSERT INTO oauth_scopes (id, scope_name, description, created_at)
        VALUES
            (gen_random_uuid(), 'read:profile', 'Чтение профиля пользователя', NOW()),
            (gen_random_uuid(), 'write:profile', 'Изменение профиля пользователя', NOW()),
            (gen_random_uuid(), 'read:email', 'Чтение email адреса', NOW()),
            (gen_random_uuid(), 'admin', 'Административный доступ', NOW())
        ON CONFLICT (scope_name) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    println!("Миграции успешно применены");
    Ok(())
}

