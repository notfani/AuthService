use axum::{Json, http::StatusCode, extract::Query, Extension};
use serde::Deserialize;
use uuid::Uuid;
use std::sync::Arc;
use sqlx::PgPool;
use chrono;

#[derive(Deserialize)]
pub struct GetHistoryQuery {
    chat_id: Uuid,
    chat_type: String, // "group" or "private"
}

#[derive(Deserialize)]
pub struct PrivateMessageRequest {
    receiver_id: Uuid,
    content: String,
}

#[derive(Deserialize)]
pub struct GroupMessageRequest {
    group_id: Uuid,
    content: String,
}

pub async fn get_message_history(
    Extension(pool): Extension<Arc<PgPool>>,
    Extension(user_id): Extension<Uuid>, // Extracted from middleware
    Query(params): Query<GetHistoryQuery>,
) -> Result<Json<Vec<crate::models::message::Message>>, StatusCode> {
    let history = match params.chat_type.as_str() {
        "group" => {
            // Проверяем, состоит ли пользователь в группе
            let is_member = sqlx::query!(
                "SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2",
                params.chat_id,
                user_id
            )
                .fetch_optional(&*pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .is_some();

            if !is_member {
                return Err(StatusCode::FORBIDDEN);
            }

            sqlx::query_as!(
                crate::models::message::Message,
                r#"
                SELECT id, sender_id, content, sent_at
                FROM messages
                WHERE group_id = $1
                ORDER BY sent_at DESC
                LIMIT 50
                "#,
                params.chat_id
            )
                .fetch_all(&*pool)
                .await
        }
        "private" => {
            // Проверяем, что это личный чат между двумя пользователями
            sqlx::query_as!(
                crate::models::message::Message,
                r#"
                SELECT id, sender_id, content, sent_at
                FROM messages
                WHERE (sender_id = $1 AND receiver_id = $2) OR (sender_id = $2 AND receiver_id = $1)
                ORDER BY sent_at DESC
                LIMIT 50
                "#,
                user_id,
                params.chat_id
            )
                .fetch_all(&*pool)
                .await
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    }
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(history))
}

pub async fn send_private_message(
    Extension(pool): Extension<Arc<PgPool>>,
    Extension(user_id): Extension<Uuid>, // Extracted from middleware
    Json(payload): Json<PrivateMessageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sent_at = chrono::Utc::now();

    sqlx::query!(
        "INSERT INTO messages (sender_id, receiver_id, content, sent_at) VALUES ($1, $2, $3, $4)",
        user_id,
        payload.receiver_id,
        payload.content,
        sent_at
    )
        .execute(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"message": "Private message sent"})))
}

pub async fn send_group_message(
    Extension(pool): Extension<Arc<PgPool>>,
    Extension(user_id): Extension<Uuid>, // Extracted from middleware
    Json(payload): Json<GroupMessageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sent_at = chrono::Utc::now();

    // Проверяем, состоит ли пользователь в группе
    let is_member = sqlx::query!(
        "SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2",
        payload.group_id,
        user_id
    )
        .fetch_optional(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    if !is_member {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query!(
        "INSERT INTO messages (sender_id, group_id, content, sent_at) VALUES ($1, $2, $3, $4)",
        user_id,
        payload.group_id,
        payload.content,
        sent_at
    )
        .execute(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"message": "Group message sent"})))
}