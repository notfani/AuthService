use axum::{
    extract::{Extension, WebSocketUpgrade, Query},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

// Глобальное хранилище WebSocket-соединений (в реальном приложении используй Redis)
type UserSockets = Arc<RwLock<std::collections::HashMap<Uuid, broadcast::Sender<String>>>>;

pub struct AppState {
    pub pool: Arc<PgPool>,
    pub user_sockets: UserSockets,
}

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
}

#[derive(Deserialize, Debug)]
struct ClientMessage {
    r#type: String, // "private", "group"
    to: Option<Uuid>, // user_id or group_id
    content: String,
}

#[derive(Serialize)]
struct ServerMessage {
    from: Uuid,
    content: String,
    sent_at: chrono::DateTime<chrono::Utc>,
}

pub async fn handle_ws(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Response {
    // Verify JWT token
    let user_id = crate::mw::auth::verify_jwt(&params.token).await;

    if user_id.is_none() {
        return (axum::http::StatusCode::UNAUTHORIZED, "Invalid token").into();
    }

    let user_id = user_id.unwrap().parse::<Uuid>().unwrap();

    ws.on_upgrade(|socket| handle_socket(socket, state, user_id))
}

async fn handle_socket(
    mut socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
    user_id: Uuid, // The authenticated user
) {
    let (mut sender, mut receiver) = socket.split();

    // Создаём broadcast-канал для этого пользователя
    let (tx, mut rx) = broadcast::channel::<String>(100);

    // Сохраняем сокет пользователя
    {
        let mut sockets = state.user_sockets.write().await;
        sockets.insert(user_id, tx.clone());
    }

    // Таск для получения сообщений от других пользователей
    let mut sender_clone = sender.sink_map_err(|e| e);
    tokio::task::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let _ = sender_clone.send(axum::extract::ws::Message::Text(msg)).await;
        }
    });

    // Обработка входящих сообщений
    while let Some(Ok(msg)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                let sent_at = chrono::Utc::now();

                let mut tx = state.pool.begin().await.unwrap();

                match client_msg.r#type.as_str() {
                    "private" => {
                        if let Some(receiver_id) = client_msg.to {
                            sqlx::query!(
                                "INSERT INTO messages (sender_id, receiver_id, content, sent_at) VALUES ($1, $2, $3, $4)",
                                user_id,
                                receiver_id,
                                &client_msg.content,
                                sent_at
                            )
                                .execute(&mut *tx)
                                .await
                                .unwrap();

                            // Отправляем сообщение получателю
                            if let Some(receiver_tx) = {
                                state.user_sockets.read().await.get(&receiver_id).cloned()
                            } {
                                let server_msg = ServerMessage {
                                    from: user_id,
                                    content: client_msg.content,
                                    sent_at,
                                };
                                if let Ok(json) = serde_json::to_string(&server_msg) {
                                    let _ = receiver_tx.send(json);
                                }
                            }
                        }
                    }
                    "group" => {
                        if let Some(group_id) = client_msg.to {
                            sqlx::query!(
                                "INSERT INTO messages (sender_id, group_id, content, sent_at) VALUES ($1, $2, $3, $4)",
                                user_id,
                                group_id,
                                &client_msg.content,
                                sent_at
                            )
                                .execute(&mut *tx)
                                .await
                                .unwrap();

                            // Отправляем сообщение всем участникам группы
                            let members = sqlx::query!("SELECT user_id FROM group_members WHERE group_id = $1", group_id)
                                .fetch_all(&*state.pool)
                                .await
                                .unwrap();

                            for member in members {
                                if let Some(member_tx) = {
                                    state.user_sockets.read().await.get(&member.user_id).cloned()
                                } {
                                    let server_msg = ServerMessage {
                                        from: user_id,
                                        content: client_msg.content.clone(),
                                        sent_at,
                                    };
                                    if let Ok(json) = serde_json::to_string(&server_msg) {
                                        let _ = member_tx.send(json);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                tx.commit().await.unwrap();
            }
        }
    }

    // Удаляем сокет при отключении
    {
        let mut sockets = state.user_sockets.write().await;
        sockets.remove(&user_id);
    }
}