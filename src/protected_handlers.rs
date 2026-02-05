use actix_web::{web, HttpRequest, HttpResponse, Responder};
use crate::middleware::get_claims_from_request;
use crate::models::ErrorResponse;

// Protected endpoint - требует аутентификации
pub async fn protected_profile(req: HttpRequest) -> impl Responder {
    match get_claims_from_request(&req) {
        Some(claims) => {
            HttpResponse::Ok().json(serde_json::json!({
                "message": "This is a protected resource",
                "user_id": claims.sub,
                "client_id": claims.client_id,
                "scopes": claims.scope,
            }))
        }
        None => HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Authentication required".to_string(),
        }),
    }
}

// Protected endpoint - требует определенный scope
pub async fn protected_data(req: HttpRequest) -> impl Responder {
    match get_claims_from_request(&req) {
        Some(claims) => {
            // Проверка наличия определенного scope
            let scopes: Vec<&str> = claims.scope.split_whitespace().collect();

            if scopes.contains(&"read:profile") {
                HttpResponse::Ok().json(serde_json::json!({
                    "message": "This is sensitive data",
                    "data": {
                        "user_id": claims.sub,
                        "permissions": scopes,
                    }
                }))
            } else {
                HttpResponse::Forbidden().json(ErrorResponse {
                    error: "Insufficient permissions. Required scope: read:profile".to_string(),
                })
            }
        }
        None => HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Authentication required".to_string(),
        }),
    }
}

// Конфигурация защищенных маршрутов
pub fn configure_protected_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/profile", web::get().to(protected_profile))
       .route("/data", web::get().to(protected_data));
}
