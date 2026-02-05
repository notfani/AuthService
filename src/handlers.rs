use actix_web::{web, HttpResponse, Responder};
use validator::Validate;
use crate::models::{RegisterUserRequest, RegisterUserResponse, ErrorResponse};
use crate::services::{UserService, RegistrationError};

// Endpoint для регистрации пользователя
pub async fn register(
    user_service: web::Data<UserService>,
    request: web::Json<RegisterUserRequest>,
) -> impl Responder {
    // Валидация входных данных
    if let Err(errors) = request.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("Ошибка валидации: {}", errors),
        });
    }

    // Регистрация пользователя
    match user_service.register_user(request.into_inner()).await {
        Ok(user) => {
            let response: RegisterUserResponse = user.into();
            HttpResponse::Created().json(response)
        }
        Err(RegistrationError::UsernameExists) => {
            HttpResponse::Conflict().json(ErrorResponse {
                error: "Пользователь с таким именем уже существует".to_string(),
            })
        }
        Err(RegistrationError::EmailExists) => {
            HttpResponse::Conflict().json(ErrorResponse {
                error: "Пользователь с таким email уже существует".to_string(),
            })
        }
        Err(e) => {
            eprintln!("Ошибка при регистрации: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Внутренняя ошибка сервера".to_string(),
            })
        }
    }
}

// Endpoint для проверки работоспособности
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "message": "Server is running"
    }))
}

// Конфигурация маршрутов
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health_check))
            .route("/register", web::post().to(register))
    );
}

