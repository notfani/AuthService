use axum::{
    extract::Extension,
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::services::ServeDir;

mod models;
mod handlers;
mod ws;
mod mw {
    pub mod auth;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let shared_state = Arc::new(ws::AppState {
        pool: Arc::new(pool),
        user_sockets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
    });

    let protected_routes = Router::new()
        .route("/messages/private", post(handlers::messages::send_private_message))
        .route("/messages/group", post(handlers::messages::send_group_message))
        .route("/messages/history", get(handlers::messages::get_message_history))
        .route("/groups", post(handlers::groups::create_group))
        .layer(middleware::from_fn_with_state(shared_state.clone(), mw::auth::require_auth));

    let app = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
        .merge(protected_routes)
        .route("/ws", get(ws::handle_ws))
        .nest_service("/", ServeDir::new("public"))
        .layer(Extension(shared_state.clone()));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}