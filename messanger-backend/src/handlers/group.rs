use axum::{Json, http::StatusCode, Extension};
use serde::Deserialize;
use uuid::Uuid;
use std::sync::Arc;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct CreateGroupRequest {
    name: String,
}

pub async fn create_group(
    Extension(pool): Extension<Arc<PgPool>>,
    Json(payload): Json<CreateGroupRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let owner_id = uuid::Uuid::new_v4(); // In real app, extract from JWT

    let group_id = sqlx::query!(
        "INSERT INTO groups (name, owner_id) VALUES ($1, $2) RETURNING id",
        payload.name,
        owner_id
    )
        .fetch_one(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .id;

    Ok(Json(serde_json::json!({"group_id": group_id})))
}