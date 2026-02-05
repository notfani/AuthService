use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{Utc, Duration};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use crate::models::{AuthorizationCode, TokenResponse, OAuthClient};
use crate::token_service::TokenService;

#[derive(Debug)]
pub enum OAuthError {
    DatabaseError(sqlx::Error),
    InvalidClient,
    InvalidGrant,
    InvalidRequest,
    UnauthorizedClient,
    UnsupportedGrantType,
    InvalidScope,
    CodeExpired,
    CodeAlreadyUsed,
    InvalidCodeVerifier,
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthError::DatabaseError(e) => write!(f, "Database error: {}", e),
            OAuthError::InvalidClient => write!(f, "invalid_client"),
            OAuthError::InvalidGrant => write!(f, "invalid_grant"),
            OAuthError::InvalidRequest => write!(f, "invalid_request"),
            OAuthError::UnauthorizedClient => write!(f, "unauthorized_client"),
            OAuthError::UnsupportedGrantType => write!(f, "unsupported_grant_type"),
            OAuthError::InvalidScope => write!(f, "invalid_scope"),
            OAuthError::CodeExpired => write!(f, "Code expired"),
            OAuthError::CodeAlreadyUsed => write!(f, "Code already used"),
            OAuthError::InvalidCodeVerifier => write!(f, "Invalid code verifier"),
        }
    }
}

impl std::error::Error for OAuthError {}

pub struct OAuthService {
    pool: Pool<Postgres>,
    token_service: TokenService,
}

impl OAuthService {
    pub fn new(pool: Pool<Postgres>, token_service: TokenService) -> Self {
        Self { pool, token_service }
    }

    // Генерация authorization code
    fn generate_authorization_code() -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    }

    // Создание authorization code в БД
    pub async fn create_authorization_code(
        &self,
        client_id: &str,
        user_id: Uuid,
        redirect_uri: &str,
        scope: &str,
        code_challenge: Option<String>,
        code_challenge_method: Option<String>,
    ) -> Result<AuthorizationCode, OAuthError> {
        let code = Self::generate_authorization_code();
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::minutes(10); // Authorization code valid for 10 minutes

        let auth_code = sqlx::query_as::<_, AuthorizationCode>(
            r#"
            INSERT INTO oauth_authorization_codes (
                id, code, client_id, user_id, redirect_uri, scope,
                code_challenge, code_challenge_method, expires_at, used, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, code, client_id, user_id, redirect_uri, scope,
                      code_challenge, code_challenge_method, expires_at, used, created_at
            "#
        )
        .bind(id)
        .bind(&code)
        .bind(client_id)
        .bind(user_id)
        .bind(redirect_uri)
        .bind(scope)
        .bind(code_challenge)
        .bind(code_challenge_method)
        .bind(expires_at)
        .bind(false)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(OAuthError::DatabaseError)?;

        Ok(auth_code)
    }

    // Получение authorization code
    async fn get_authorization_code(&self, code: &str) -> Result<Option<AuthorizationCode>, OAuthError> {
        let auth_code = sqlx::query_as::<_, AuthorizationCode>(
            r#"
            SELECT id, code, client_id, user_id, redirect_uri, scope,
                   code_challenge, code_challenge_method, expires_at, used, created_at
            FROM oauth_authorization_codes
            WHERE code = $1
            "#
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(OAuthError::DatabaseError)?;

        Ok(auth_code)
    }

    // Пометить код как использованный
    async fn mark_code_as_used(&self, code: &str) -> Result<(), OAuthError> {
        sqlx::query("UPDATE oauth_authorization_codes SET used = true WHERE code = $1")
            .bind(code)
            .execute(&self.pool)
            .await
            .map_err(OAuthError::DatabaseError)?;

        Ok(())
    }

    // Проверка PKCE code_verifier
    fn verify_pkce(&self, code_verifier: &str, code_challenge: &str, method: &str) -> bool {
        match method {
            "S256" => {
                let mut hasher = Sha256::new();
                hasher.update(code_verifier.as_bytes());
                let result = hasher.finalize();
                let computed_challenge = general_purpose::URL_SAFE_NO_PAD.encode(result);
                computed_challenge == code_challenge
            }
            "plain" => code_verifier == code_challenge,
            _ => false,
        }
    }

    // Обмен authorization code на токены (Authorization Code Flow)
    pub async fn exchange_code_for_tokens(
        &self,
        code: &str,
        client: &OAuthClient,
        redirect_uri: &str,
        code_verifier: Option<String>,
    ) -> Result<TokenResponse, OAuthError> {
        // Получение кода
        let auth_code = self.get_authorization_code(code)
            .await?
            .ok_or(OAuthError::InvalidGrant)?;

        // Проверки
        if auth_code.used {
            return Err(OAuthError::CodeAlreadyUsed);
        }

        if Utc::now() > auth_code.expires_at {
            return Err(OAuthError::CodeExpired);
        }

        if auth_code.client_id != client.client_id {
            return Err(OAuthError::InvalidClient);
        }

        if auth_code.redirect_uri != redirect_uri {
            return Err(OAuthError::InvalidGrant);
        }

        // Проверка PKCE если использовался
        if let Some(challenge) = &auth_code.code_challenge {
            let verifier = code_verifier.ok_or(OAuthError::InvalidRequest)?;
            let method = auth_code.code_challenge_method.as_deref().unwrap_or("plain");

            if !self.verify_pkce(&verifier, challenge, method) {
                return Err(OAuthError::InvalidCodeVerifier);
            }
        }

        // Пометить код как использованный
        self.mark_code_as_used(code).await?;

        // Генерация токенов
        let access_token = self.token_service.create_jwt(
            Some(auth_code.user_id),
            &client.client_id,
            &auth_code.scope,
        ).map_err(|_| OAuthError::InvalidRequest)?;

        let refresh_token = self.token_service.generate_refresh_token();

        // Сохранение токенов в БД
        self.token_service.store_tokens(
            &access_token,
            Some(&refresh_token),
            &client.client_id,
            Some(auth_code.user_id),
            &auth_code.scope,
        ).await.map_err(OAuthError::DatabaseError)?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.token_service.get_access_token_ttl(),
            refresh_token: Some(refresh_token),
            scope: auth_code.scope,
        })
    }

    // Client Credentials Flow
    pub async fn issue_client_credentials_token(
        &self,
        client: &OAuthClient,
        scope: Option<&str>,
    ) -> Result<TokenResponse, OAuthError> {
        let scope = scope.unwrap_or("").to_string();

        // Генерация access token
        let access_token = self.token_service.create_jwt(
            None,
            &client.client_id,
            &scope,
        ).map_err(|_| OAuthError::InvalidRequest)?;

        // Сохранение токена в БД (без refresh token для client credentials)
        self.token_service.store_tokens(
            &access_token,
            None,
            &client.client_id,
            None,
            &scope,
        ).await.map_err(OAuthError::DatabaseError)?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.token_service.get_access_token_ttl(),
            refresh_token: None,
            scope,
        })
    }

    // Refresh Token Flow
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
        client: &OAuthClient,
    ) -> Result<TokenResponse, OAuthError> {
        // Получение старого токена
        let old_token = self.token_service.get_token_by_refresh(refresh_token)
            .await
            .map_err(OAuthError::DatabaseError)?
            .ok_or(OAuthError::InvalidGrant)?;

        // Проверка client_id
        if old_token.client_id != client.client_id {
            return Err(OAuthError::InvalidClient);
        }

        // Отзыв старого токена
        self.token_service.revoke_token(&old_token.access_token)
            .await
            .map_err(OAuthError::DatabaseError)?;

        // Генерация новых токенов
        let new_access_token = self.token_service.create_jwt(
            old_token.user_id,
            &client.client_id,
            &old_token.scope,
        ).map_err(|_| OAuthError::InvalidRequest)?;

        let new_refresh_token = self.token_service.generate_refresh_token();

        // Сохранение новых токенов
        self.token_service.store_tokens(
            &new_access_token,
            Some(&new_refresh_token),
            &client.client_id,
            old_token.user_id,
            &old_token.scope,
        ).await.map_err(OAuthError::DatabaseError)?;

        Ok(TokenResponse {
            access_token: new_access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.token_service.get_access_token_ttl(),
            refresh_token: Some(new_refresh_token),
            scope: old_token.scope,
        })
    }
}
