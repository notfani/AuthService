pub mod models;
pub mod services;
pub mod handlers;
pub mod database;
pub mod token_service;
pub mod client_service;
pub mod oauth_service;
pub mod auth_handlers;
pub mod oauth_handlers;
pub mod middleware;
pub mod protected_handlers;

use actix_web::{App, HttpServer, web, middleware as actix_middleware};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use dotenv::dotenv;
use std::env;
use services::UserService;
use token_service::TokenService;
use client_service::ClientService;
use oauth_service::OAuthService;
use middleware::AuthMiddleware;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Загрузка переменных окружения
    dotenv().ok();

    // Получение DATABASE_URL из переменных окружения
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            println!("DATABASE_URL не найден в .env файле, используется значение по умолчанию");
            "postgresql://postgres:password@localhost:5432/mirea_backend".to_string()
        });

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
        println!("WARNING: Using default JWT_SECRET. Set JWT_SECRET in .env for production!");
        "your-secret-key-change-this-in-production".to_string()
    });
    let session_key = env::var("SESSION_KEY").unwrap_or_else(|_| {
        println!("WARNING: Using default SESSION_KEY. Set SESSION_KEY in .env for production!");
        "your-session-key-must-be-at-least-64-bytes-long-change-this-in-prod".to_string()
    });

    println!("Подключение к базе данных...");

    // Создание пула подключений к БД
    let pool = database::create_pool(&database_url)
        .await
        .expect("Не удалось подключиться к базе данных");

    println!("Подключение к базе данных установлено");

    // Запуск миграций
    println!("Применение миграций...");
    database::run_migrations(&pool)
        .await
        .expect("Не удалось применить миграции");

    // Создание сервисов
    let user_service = web::Data::new(UserService::new(pool.clone()));
    let token_service = TokenService::new(pool.clone(), jwt_secret);
    let token_service_data = web::Data::new(token_service);
    let client_service = web::Data::new(ClientService::new(pool.clone()));
    let oauth_service = web::Data::new(OAuthService::new(
        pool.clone(),
        TokenService::new(pool.clone(), env::var("JWT_SECRET").unwrap_or_else(|_| {
            "your-secret-key-change-this-in-production".to_string()
        })),
    ));

    // Создание session key
    let secret_key = Key::from(session_key.as_bytes());

    let bind_address = format!("{}:{}", host, port);
    println!("OAuth 2.0 сервер запущен на http://{}", bind_address);
    println!("\n=== API Endpoints ===");
    println!("Health Check:");
    println!("  GET  http://{}/api/health", bind_address);
    println!("\nUser Management:");
    println!("  POST http://{}/api/register", bind_address);
    println!("\nAuthentication:");
    println!("  GET  http://{}/auth/login", bind_address);
    println!("  POST http://{}/auth/login", bind_address);
    println!("  POST http://{}/auth/logout", bind_address);
    println!("  GET  http://{}/auth/me", bind_address);
    println!("\nOAuth 2.0:");
    println!("  GET  http://{}/oauth/authorize", bind_address);
    println!("  POST http://{}/oauth/authorize", bind_address);
    println!("  POST http://{}/oauth/token", bind_address);
    println!("  POST http://{}/oauth/revoke", bind_address);
    println!("  POST http://{}/oauth/clients", bind_address);
    println!("\nProtected Resources:");
    println!("  GET  http://{}/api/protected/profile", bind_address);
    println!("  GET  http://{}/api/protected/data", bind_address);
    println!("\n===================\n");

    // Запуск HTTP сервера
    HttpServer::new(move || {
        let token_service_for_middleware = TokenService::new(
            pool.clone(),
            env::var("JWT_SECRET").unwrap_or_else(|_| {
                "your-secret-key-change-this-in-production".to_string()
            })
        );

        App::new()
            .app_data(user_service.clone())
            .app_data(token_service_data.clone())
            .app_data(client_service.clone())
            .app_data(oauth_service.clone())
            .wrap(actix_middleware::Logger::default())
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                )
                .cookie_secure(false) // Set to true in production with HTTPS
                .build()
            )
            .configure(handlers::configure_routes)
            .configure(auth_handlers::configure_auth_routes)
            .configure(oauth_handlers::configure_oauth_routes)
            .service(
                web::scope("/api/protected")
                    .wrap(AuthMiddleware::new(token_service_for_middleware))
                    .configure(protected_handlers::configure_protected_routes)
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}
