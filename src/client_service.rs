use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::Utc;
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use crate::models::{OAuthClient, CreateClientRequest};

#[derive(Debug)]
pub enum ClientError {
    DatabaseError(sqlx::Error),
    ClientNotFound,
    InvalidCredentials,
    InvalidRedirectUri,
    InvalidScope,
    InvalidGrantType,
    HashError,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::DatabaseError(e) => write!(f, "Database error: {}", e),
            ClientError::ClientNotFound => write!(f, "Client not found"),
            ClientError::InvalidCredentials => write!(f, "Invalid client credentials"),
            ClientError::InvalidRedirectUri => write!(f, "Invalid redirect URI"),
            ClientError::InvalidScope => write!(f, "Invalid scope"),
            ClientError::InvalidGrantType => write!(f, "Invalid grant type"),
            ClientError::HashError => write!(f, "Error hashing client secret"),
        }
    }
}

impl std::error::Error for ClientError {}

pub struct ClientService {
    pool: Pool<Postgres>,
}

impl ClientService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    // Генерация client_id
    fn generate_client_id() -> String {
        let random_part: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        format!("client_{}", random_part)
    }

    // Генерация client_secret
    fn generate_client_secret() -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    }

    // Регистрация нового OAuth клиента
    pub async fn register_client(&self, request: CreateClientRequest) -> Result<(OAuthClient, String), ClientError> {
        let client_id = Self::generate_client_id();
        let client_secret = Self::generate_client_secret();
        let client_secret_hash = hash(&client_secret, DEFAULT_COST)
            .map_err(|_| ClientError::HashError)?;

        let id = Uuid::new_v4();
        let now = Utc::now();

        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            INSERT INTO oauth_clients (
                id, client_id, client_secret_hash, client_name, redirect_uris,
                allowed_scopes, grant_types, is_confidential, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, client_id, client_secret_hash, client_name, redirect_uris,
                      allowed_scopes, grant_types, is_confidential, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(&client_id)
        .bind(&client_secret_hash)
        .bind(&request.client_name)
        .bind(&request.redirect_uris)
        .bind(&request.allowed_scopes)
        .bind(&request.grant_types)
        .bind(request.is_confidential)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(ClientError::DatabaseError)?;

        Ok((client, client_secret))
    }

    // Получение клиента по client_id
    pub async fn get_client_by_id(&self, client_id: &str) -> Result<Option<OAuthClient>, ClientError> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            SELECT id, client_id, client_secret_hash, client_name, redirect_uris,
                   allowed_scopes, grant_types, is_confidential, created_at, updated_at
            FROM oauth_clients
            WHERE client_id = $1
            "#
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(ClientError::DatabaseError)?;

        Ok(client)
    }

    // Валидация client credentials
    pub async fn validate_client_credentials(&self, client_id: &str, client_secret: &str) -> Result<OAuthClient, ClientError> {
        let client = self.get_client_by_id(client_id)
            .await?
            .ok_or(ClientError::ClientNotFound)?;

        if !client.is_confidential {
            return Ok(client);
        }

        let is_valid = verify(client_secret, &client.client_secret_hash)
            .map_err(|_| ClientError::InvalidCredentials)?;

        if !is_valid {
            return Err(ClientError::InvalidCredentials);
        }

        Ok(client)
    }

    // Валидация redirect_uri
    pub fn validate_redirect_uri(&self, client: &OAuthClient, redirect_uri: &str) -> Result<(), ClientError> {
        if !client.redirect_uris.contains(&redirect_uri.to_string()) {
            return Err(ClientError::InvalidRedirectUri);
        }
        Ok(())
    }

    // Валидация scope
    pub fn validate_scope(&self, client: &OAuthClient, scope: &str) -> Result<(), ClientError> {
        let requested_scopes: Vec<&str> = scope.split_whitespace().collect();

        for requested_scope in requested_scopes {
            if !client.allowed_scopes.contains(&requested_scope.to_string()) {
                return Err(ClientError::InvalidScope);
            }
        }

        Ok(())
    }

    // Валидация grant_type
    pub fn validate_grant_type(&self, client: &OAuthClient, grant_type: &str) -> Result<(), ClientError> {
        if !client.grant_types.contains(&grant_type.to_string()) {
            return Err(ClientError::InvalidGrantType);
        }
        Ok(())
    }

    // Удаление клиента
    pub async fn delete_client(&self, client_id: &str) -> Result<bool, ClientError> {
        let result = sqlx::query("DELETE FROM oauth_clients WHERE client_id = $1")
            .bind(client_id)
            .execute(&self.pool)
            .await
            .map_err(ClientError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }
}
