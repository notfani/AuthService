pub mod models;
pub mod services;
pub mod handlers;
pub mod database;

use actix_web::{App, HttpServer, web, middleware};
use dotenv::dotenv;
use std::env;
use services::UserService;

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

    // Создание сервиса пользователей
    let user_service = web::Data::new(UserService::new(pool.clone()));

    let bind_address = format!("{}:{}", host, port);
    println!("Сервер запущен на http://{}", bind_address);
    println!("API endpoints:");
    println!("GET  http://{}/api/health - проверка работоспособности", bind_address);
    println!("POST http://{}/api/register - регистрация пользователя", bind_address);

    // Запуск HTTP сервера
    HttpServer::new(move || {
        App::new()
            .app_data(user_service.clone())
            .wrap(middleware::Logger::default())
            .configure(handlers::configure_routes)
    })
    .bind(&bind_address)?
    .run()
    .await
}
