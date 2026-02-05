use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
    body::MessageBody,
};
use futures::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;
use crate::token_service::TokenService;
use crate::models::TokenClaims;

// Middleware для проверки Bearer токенов
pub struct AuthMiddleware {
    token_service: Rc<TokenService>,
}

impl AuthMiddleware {
    pub fn new(token_service: TokenService) -> Self {
        Self {
            token_service: Rc::new(token_service),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
            token_service: self.token_service.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
    token_service: Rc<TokenService>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let token_service = self.token_service.clone();

        Box::pin(async move {
            // Извлечение токена из заголовка Authorization
            let token = match extract_bearer_token(&req) {
                Some(t) => t,
                None => {
                    let (http_req, _) = req.into_parts();
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Missing or invalid Authorization header"
                        }));
                    return Ok(ServiceResponse::new(http_req, response).map_into_boxed_body());
                }
            };

            // Верификация JWT
            let claims = match token_service.verify_jwt(&token) {
                Ok(claims) => claims,
                Err(e) => {
                    eprintln!("Token verification failed: {}", e);
                    let (http_req, _) = req.into_parts();
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Invalid or expired token"
                        }));
                    return Ok(ServiceResponse::new(http_req, response).map_into_boxed_body());
                }
            };

            // Проверка токена в БД (не отозван ли)
            match token_service.validate_token(&token).await {
                Ok(Some(_)) => {
                    // Токен валиден, добавляем claims в extensions
                    req.extensions_mut().insert(claims);
                    let res = service.call(req).await?;
                    Ok(res.map_into_boxed_body())
                }
                Ok(None) => {
                    // Токен не найден или отозван
                    let (http_req, _) = req.into_parts();
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Token has been revoked"
                        }));
                    Ok(ServiceResponse::new(http_req, response).map_into_boxed_body())
                }
                Err(e) => {
                    eprintln!("Database error during token validation: {}", e);
                    let (http_req, _) = req.into_parts();
                    let response = HttpResponse::InternalServerError()
                        .json(serde_json::json!({
                            "error": "Internal server error"
                        }));
                    Ok(ServiceResponse::new(http_req, response).map_into_boxed_body())
                }
            }
        })
    }
}

// Извлечение Bearer токена из заголовка Authorization
fn extract_bearer_token(req: &ServiceRequest) -> Option<String> {
    let auth_header = req.headers().get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    if !auth_str.starts_with("Bearer ") {
        return None;
    }

    Some(auth_str[7..].to_string())
}

// Helper для извлечения claims из request
pub fn get_claims_from_request(req: &actix_web::HttpRequest) -> Option<TokenClaims> {
    req.extensions().get::<TokenClaims>().cloned()
}

// Middleware для проверки конкретных scopes
pub struct ScopeValidator {
    required_scopes: Vec<String>,
}

impl ScopeValidator {
    pub fn new(scopes: Vec<String>) -> Self {
        Self {
            required_scopes: scopes,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ScopeValidator
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = ScopeValidatorService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ScopeValidatorService {
            service: Rc::new(service),
            required_scopes: self.required_scopes.clone(),
        }))
    }
}

pub struct ScopeValidatorService<S> {
    service: Rc<S>,
    required_scopes: Vec<String>,
}

impl<S, B> Service<ServiceRequest> for ScopeValidatorService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let required_scopes = self.required_scopes.clone();

        Box::pin(async move {
            // Получение claims из extensions (должен быть установлен AuthMiddleware)
            let claims = {
                let ext = req.extensions();
                match ext.get::<TokenClaims>() {
                    Some(c) => c.clone(),
                    None => {
                        drop(ext); // Освобождаем borrow
                        let (http_req, _) = req.into_parts();
                        let response = HttpResponse::Unauthorized()
                            .json(serde_json::json!({
                                "error": "Authentication required"
                            }));
                        return Ok(ServiceResponse::new(http_req, response).map_into_boxed_body());
                    }
                }
            };

            // Проверка scopes
            let user_scopes: Vec<&str> = claims.scope.split_whitespace().collect();

            for required_scope in &required_scopes {
                if !user_scopes.contains(&required_scope.as_str()) {
                    let (http_req, _) = req.into_parts();
                    let response = HttpResponse::Forbidden()
                        .json(serde_json::json!({
                            "error": format!("Missing required scope: {}", required_scope)
                        }));
                    return Ok(ServiceResponse::new(http_req, response).map_into_boxed_body());
                }
            }

            let res = service.call(req).await?;
            Ok(res.map_into_boxed_body())
        })
    }
}
