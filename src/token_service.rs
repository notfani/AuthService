use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{Utc, Duration};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use crate::models::{TokenClaims, OAuthToken};

pub struct TokenService {
    pool: Pool<Postgres>,
    jwt_secret: String,
    access_token_ttl: i64,  // seconds
    refresh_token_ttl: i64, // seconds
}

impl TokenService {
    pub fn new(pool: Pool<Postgres>, jwt_secret: String) -> Self {
        Self {
            pool,
            jwt_secret,
            access_token_ttl: 3600,        // 1 hour
            refresh_token_ttl: 2592000,    // 30 days
        }
    }

    // Генерация JWT access token
    pub fn create_jwt(&self, user_id: Option<Uuid>, client_id: &str, scope: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now().timestamp();
        let exp = now + self.access_token_ttl;

        let claims = TokenClaims {
            sub: user_id.map(|id| id.to_string()).unwrap_or_else(|| client_id.to_string()),
            client_id: client_id.to_string(),
            scope: scope.to_string(),
            exp,
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(token)
    }

    // Верификация и декодирование JWT
    pub fn verify_jwt(&self, token: &str) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<TokenClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )?;

        Ok(token_data.claims)
    }

    // Генерация случайного refresh token
    pub fn generate_refresh_token(&self) -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    }

    // Сохранение токенов в БД
    pub async fn store_tokens(
        &self,
        access_token: &str,
        refresh_token: Option<&str>,
        client_id: &str,
        user_id: Option<Uuid>,
        scope: &str,
    ) -> Result<OAuthToken, sqlx::Error> {
        let token_id = Uuid::new_v4();
        let now = Utc::now();
        let access_expires_at = now + Duration::seconds(self.access_token_ttl);
        let refresh_expires_at = refresh_token.map(|_| now + Duration::seconds(self.refresh_token_ttl));

        let token = sqlx::query_as::<_, OAuthToken>(
            r#"
            INSERT INTO oauth_tokens (
                id, access_token, refresh_token, client_id, user_id, scope,
                token_type, expires_at, refresh_expires_at, revoked, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, access_token, refresh_token, client_id, user_id, scope,
                      token_type, expires_at, refresh_expires_at, revoked, created_at
            "#
        )
        .bind(token_id)
        .bind(access_token)
        .bind(refresh_token)
        .bind(client_id)
        .bind(user_id)
        .bind(scope)
        .bind("Bearer")
        .bind(access_expires_at)
        .bind(refresh_expires_at)
        .bind(false)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    // Проверка токена в БД (не отозван ли)
    pub async fn validate_token(&self, access_token: &str) -> Result<Option<OAuthToken>, sqlx::Error> {
        let token = sqlx::query_as::<_, OAuthToken>(
            r#"
            SELECT id, access_token, refresh_token, client_id, user_id, scope,
                   token_type, expires_at, refresh_expires_at, revoked, created_at
            FROM oauth_tokens
            WHERE access_token = $1 AND revoked = false AND expires_at > NOW()
            "#
        )
        .bind(access_token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    // Получение токена по refresh_token
    pub async fn get_token_by_refresh(&self, refresh_token: &str) -> Result<Option<OAuthToken>, sqlx::Error> {
        let token = sqlx::query_as::<_, OAuthToken>(
            r#"
            SELECT id, access_token, refresh_token, client_id, user_id, scope,
                   token_type, expires_at, refresh_expires_at, revoked, created_at
            FROM oauth_tokens
            WHERE refresh_token = $1 AND revoked = false AND refresh_expires_at > NOW()
            "#
        )
        .bind(refresh_token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    // Отзыв токена
    pub async fn revoke_token(&self, token: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_tokens
            SET revoked = true
            WHERE access_token = $1 OR refresh_token = $1
            "#
        )
        .bind(token)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // Очистка истекших токенов
    pub async fn cleanup_expired_tokens(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_tokens
            WHERE expires_at < NOW() OR (refresh_expires_at IS NOT NULL AND refresh_expires_at < NOW())
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub fn get_access_token_ttl(&self) -> i64 {
        self.access_token_ttl
    }
}
