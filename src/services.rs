use sqlx::{Pool, Postgres};
use crate::models::{RegisterUserRequest, User};
use bcrypt::{hash, DEFAULT_COST};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub enum RegistrationError {
    DatabaseError(sqlx::Error),
    UsernameExists,
    EmailExists,
    HashError,
}

impl std::fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationError::DatabaseError(e) => write!(f, "Ошибка базы данных: {}", e),
            RegistrationError::UsernameExists => write!(f, "Пользователь с таким именем уже существует"),
            RegistrationError::EmailExists => write!(f, "Пользователь с таким email уже существует"),
            RegistrationError::HashError => write!(f, "Ошибка хеширования пароля"),
        }
    }
}

impl std::error::Error for RegistrationError {}

pub struct UserService {
    pool: Pool<Postgres>,
}

impl UserService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    // Проверка существования пользователя по username
    async fn username_exists(&self, username: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)"
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    // Проверка существования пользователя по email
    async fn email_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)"
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    // Регистрация нового пользователя
    pub async fn register_user(&self, request: RegisterUserRequest) -> Result<User, RegistrationError> {
        // Проверка на существование username
        if self.username_exists(&request.username).await.map_err(RegistrationError::DatabaseError)? {
            return Err(RegistrationError::UsernameExists);
        }

        // Проверка на существование email
        if self.email_exists(&request.email).await.map_err(RegistrationError::DatabaseError)? {
            return Err(RegistrationError::EmailExists);
        }

        // Хеширование пароля
        let password_hash = hash(&request.password, DEFAULT_COST)
            .map_err(|_| RegistrationError::HashError)?;

        // Создание нового пользователя
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, username, email, password_hash, created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(&password_hash)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(RegistrationError::DatabaseError)?;

        Ok(user)
    }

    // Получение пользователя по ID
    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    // Получение пользователя по email (для авторизации)
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }
}

