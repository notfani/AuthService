use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// Модель пользователя в базе данных
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// DTO для регистрации пользователя
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterUserRequest {
    #[validate(length(min = 3, max = 50, message = "Имя пользователя должно быть от 3 до 50 символов"))]
    pub username: String,

    #[validate(email(message = "Некорректный формат email"))]
    pub email: String,

    #[validate(length(min = 8, message = "Пароль должен содержать минимум 8 символов"))]
    pub password: String,
}

// DTO для ответа после регистрации
#[derive(Debug, Serialize)]
pub struct RegisterUserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for RegisterUserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            created_at: user.created_at,
        }
    }
}

// Общий ответ об ошибке
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

