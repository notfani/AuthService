use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use crate::models::{AuthorizeRequest, ConsentRequest, TokenRequest, OAuthErrorResponse, CreateClientRequest, CreateClientResponse};
use crate::client_service::ClientService;
use crate::oauth_service::OAuthService;
use crate::token_service::TokenService;
use validator::Validate;

// GET /oauth/authorize - показывает consent screen
pub async fn authorize_get(
    query: web::Query<AuthorizeRequest>,
    client_service: web::Data<ClientService>,
    session: Session,
) -> impl Responder {
    // Проверка аутентификации пользователя
    let _user_id_str = match session.get::<String>("user_id") {
        Ok(Some(id)) => id,
        Ok(None) => {
            // Redirect to login with return URL
            let return_url = format!(
                "/oauth/authorize?response_type={}&client_id={}&redirect_uri={}&scope={}&state={}",
                query.response_type,
                query.client_id,
                urlencoding::encode(&query.redirect_uri),
                urlencoding::encode(&query.scope.as_deref().unwrap_or("")),
                urlencoding::encode(&query.state.as_deref().unwrap_or(""))
            );
            return HttpResponse::Found()
                .append_header(("Location", format!("/auth/login?return_to={}", urlencoding::encode(&return_url))))
                .finish();
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Session error");
        }
    };

    // Валидация параметров
    if query.response_type != "code" {
        return build_error_redirect(&query.redirect_uri, "unsupported_response_type", Some("Only 'code' response type is supported"), query.state.as_deref());
    }

    // Получение клиента
    let client = match client_service.get_client_by_id(&query.client_id).await {
        Ok(Some(client)) => client,
        Ok(None) => {
            return HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_client".to_string(),
                error_description: Some("Client not found".to_string()),
            });
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(OAuthErrorResponse {
                error: "server_error".to_string(),
                error_description: Some("Database error".to_string()),
            });
        }
    };

    // Валидация redirect_uri
    if let Err(_) = client_service.validate_redirect_uri(&client, &query.redirect_uri) {
        return HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_request".to_string(),
            error_description: Some("Invalid redirect_uri".to_string()),
        });
    }

    // Валидация scope
    let scope = query.scope.as_deref().unwrap_or("");
    if let Err(_) = client_service.validate_scope(&client, scope) {
        return build_error_redirect(&query.redirect_uri, "invalid_scope", Some("Requested scope not allowed"), query.state.as_deref());
    }

    // Отображение consent screen
    let scopes: Vec<&str> = scope.split_whitespace().collect();
    let scopes_html = scopes.iter()
        .map(|s| format!("<li>{}</li>", s))
        .collect::<Vec<_>>()
        .join("");

    let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Авторизация</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 500px; margin: 50px auto; padding: 20px; }}
        h1 {{ text-align: center; }}
        .client-info {{ background-color: #f5f5f5; padding: 15px; border-radius: 4px; margin-bottom: 20px; }}
        .scopes {{ margin: 20px 0; }}
        .scopes ul {{ list-style: none; padding: 0; }}
        .scopes li {{ padding: 8px; background: #e3f2fd; margin: 5px 0; border-radius: 4px; }}
        .buttons {{ display: flex; gap: 10px; justify-content: center; }}
        button {{ padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; }}
        .approve {{ background-color: #28a745; color: white; }}
        .approve:hover {{ background-color: #218838; }}
        .deny {{ background-color: #dc3545; color: white; }}
        .deny:hover {{ background-color: #c82333; }}
    </style>
</head>
<body>
    <h1>Запрос авторизации</h1>
    <div class="client-info">
        <p><strong>Приложение:</strong> {}</p>
        <p><strong>Client ID:</strong> {}</p>
    </div>
    <div class="scopes">
        <p><strong>Запрашиваемые разрешения:</strong></p>
        <ul>{}</ul>
    </div>
    <form id="consentForm" method="POST" action="/oauth/authorize">
        <input type="hidden" name="client_id" value="{}">
        <input type="hidden" name="redirect_uri" value="{}">
        <input type="hidden" name="scope" value="{}">
        <input type="hidden" name="state" value="{}">
        <input type="hidden" name="code_challenge" value="{}">
        <input type="hidden" name="code_challenge_method" value="{}">
        <input type="hidden" name="approved" value="false" id="approvedField">
        <div class="buttons">
            <button type="button" class="approve" onclick="approve()">Разрешить</button>
            <button type="button" class="deny" onclick="deny()">Отклонить</button>
        </div>
    </form>

    <script>
        function approve() {{
            document.getElementById('approvedField').value = 'true';
            document.getElementById('consentForm').submit();
        }}
        function deny() {{
            document.getElementById('approvedField').value = 'false';
            document.getElementById('consentForm').submit();
        }}
    </script>
</body>
</html>
    "#,
        client.client_name,
        client.client_id,
        scopes_html,
        query.client_id,
        query.redirect_uri,
        scope,
        query.state.as_deref().unwrap_or(""),
        query.code_challenge.as_deref().unwrap_or(""),
        query.code_challenge_method.as_deref().unwrap_or("")
    );

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

// POST /oauth/authorize - обработка согласия пользователя
pub async fn authorize_post(
    form: web::Form<ConsentRequest>,
    oauth_service: web::Data<OAuthService>,
    client_service: web::Data<ClientService>,
    session: Session,
) -> impl Responder {
    // Получение user_id из сессии
    let user_id_str = match session.get::<String>("user_id") {
        Ok(Some(id)) => id,
        _ => {
            return build_error_redirect(&form.redirect_uri, "access_denied", Some("User not authenticated"), form.state.as_deref());
        }
    };

    let user_id = match user_id_str.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return build_error_redirect(&form.redirect_uri, "server_error", Some("Invalid user ID"), form.state.as_deref());
        }
    };

    // Проверка согласия
    if !form.approved {
        return build_error_redirect(&form.redirect_uri, "access_denied", Some("User denied authorization"), form.state.as_deref());
    }

    // Получение клиента
    let _client = match client_service.get_client_by_id(&form.client_id).await {
        Ok(Some(client)) => client,
        _ => {
            return build_error_redirect(&form.redirect_uri, "invalid_client", Some("Client not found"), form.state.as_deref());
        }
    };

    // Создание authorization code
    match oauth_service.create_authorization_code(
        &form.client_id,
        user_id,
        &form.redirect_uri,
        &form.scope,
        form.code_challenge.clone(),
        form.code_challenge_method.clone(),
    ).await {
        Ok(auth_code) => {
            // Redirect обратно в приложение с кодом
            let mut redirect_url = format!("{}?code={}", form.redirect_uri, auth_code.code);
            if let Some(state) = &form.state {
                redirect_url.push_str(&format!("&state={}", state));
            }
            HttpResponse::Found()
                .append_header(("Location", redirect_url))
                .finish()
        }
        Err(_) => {
            build_error_redirect(&form.redirect_uri, "server_error", Some("Failed to create authorization code"), form.state.as_deref())
        }
    }
}

// POST /oauth/token - обмен кода/refresh token на access token
pub async fn token(
    form: web::Form<TokenRequest>,
    oauth_service: web::Data<OAuthService>,
    client_service: web::Data<ClientService>,
) -> impl Responder {
    // Получение client_id и client_secret
    let client_id = form.client_id.as_deref().unwrap_or("");
    let client_secret = form.client_secret.as_deref().unwrap_or("");

    // Валидация клиента
    let client = match client_service.validate_client_credentials(client_id, client_secret).await {
        Ok(client) => client,
        Err(_) => {
            return HttpResponse::Unauthorized().json(OAuthErrorResponse {
                error: "invalid_client".to_string(),
                error_description: Some("Invalid client credentials".to_string()),
            });
        }
    };

    // Обработка в зависимости от grant_type
    match form.grant_type.as_str() {
        "authorization_code" => {
            let code = match &form.code {
                Some(c) => c,
                None => {
                    return HttpResponse::BadRequest().json(OAuthErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some("Missing 'code' parameter".to_string()),
                    });
                }
            };

            let redirect_uri = match &form.redirect_uri {
                Some(uri) => uri,
                None => {
                    return HttpResponse::BadRequest().json(OAuthErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some("Missing 'redirect_uri' parameter".to_string()),
                    });
                }
            };

            match oauth_service.exchange_code_for_tokens(
                code,
                &client,
                redirect_uri,
                form.code_verifier.clone(),
            ).await {
                Ok(token_response) => HttpResponse::Ok().json(token_response),
                Err(e) => {
                    HttpResponse::BadRequest().json(OAuthErrorResponse {
                        error: e.to_string(),
                        error_description: Some("Failed to exchange code for tokens".to_string()),
                    })
                }
            }
        }
        "client_credentials" => {
            match oauth_service.issue_client_credentials_token(&client, form.scope.as_deref()).await {
                Ok(token_response) => HttpResponse::Ok().json(token_response),
                Err(e) => {
                    HttpResponse::BadRequest().json(OAuthErrorResponse {
                        error: e.to_string(),
                        error_description: Some("Failed to issue client credentials token".to_string()),
                    })
                }
            }
        }
        "refresh_token" => {
            let refresh_token = match &form.refresh_token {
                Some(token) => token,
                None => {
                    return HttpResponse::BadRequest().json(OAuthErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some("Missing 'refresh_token' parameter".to_string()),
                    });
                }
            };

            match oauth_service.refresh_access_token(refresh_token, &client).await {
                Ok(token_response) => HttpResponse::Ok().json(token_response),
                Err(e) => {
                    HttpResponse::BadRequest().json(OAuthErrorResponse {
                        error: e.to_string(),
                        error_description: Some("Failed to refresh token".to_string()),
                    })
                }
            }
        }
        _ => {
            HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "unsupported_grant_type".to_string(),
                error_description: Some(format!("Grant type '{}' is not supported", form.grant_type)),
            })
        }
    }
}

// POST /oauth/revoke - отзыв токена
pub async fn revoke(
    form: web::Form<serde_json::Value>,
    token_service: web::Data<TokenService>,
) -> impl Responder {
    let token = match form.get("token").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_request".to_string(),
                error_description: Some("Missing 'token' parameter".to_string()),
            });
        }
    };

    match token_service.revoke_token(token).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"message": "Token revoked"})),
        Err(_) => {
            HttpResponse::InternalServerError().json(OAuthErrorResponse {
                error: "server_error".to_string(),
                error_description: Some("Failed to revoke token".to_string()),
            })
        }
    }
}

// POST /oauth/clients - регистрация нового OAuth клиента (административный endpoint)
pub async fn register_client(
    client_service: web::Data<ClientService>,
    request: web::Json<CreateClientRequest>,
) -> impl Responder {
    // В продакшене здесь должна быть проверка прав администратора

    if let Err(errors) = request.validate() {
        return HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_request".to_string(),
            error_description: Some(format!("Validation error: {}", errors)),
        });
    }

    match client_service.register_client(request.into_inner()).await {
        Ok((client, client_secret)) => {
            HttpResponse::Created().json(CreateClientResponse {
                client_id: client.client_id,
                client_secret,
                client_name: client.client_name,
                redirect_uris: client.redirect_uris,
                allowed_scopes: client.allowed_scopes,
                grant_types: client.grant_types,
            })
        }
        Err(e) => {
            eprintln!("Error registering client: {}", e);
            HttpResponse::InternalServerError().json(OAuthErrorResponse {
                error: "server_error".to_string(),
                error_description: Some("Failed to register client".to_string()),
            })
        }
    }
}

// Helper function to build error redirect
fn build_error_redirect(redirect_uri: &str, error: &str, description: Option<&str>, state: Option<&str>) -> HttpResponse {
    let mut redirect_url = format!("{}?error={}", redirect_uri, error);

    if let Some(desc) = description {
        redirect_url.push_str(&format!("&error_description={}", urlencoding::encode(desc)));
    }

    if let Some(state) = state {
        redirect_url.push_str(&format!("&state={}", state));
    }

    HttpResponse::Found()
        .append_header(("Location", redirect_url))
        .finish()
}

// Конфигурация маршрутов для OAuth
pub fn configure_oauth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/oauth")
            .route("/authorize", web::get().to(authorize_get))
            .route("/authorize", web::post().to(authorize_post))
            .route("/token", web::post().to(token))
            .route("/revoke", web::post().to(revoke))
            .route("/clients", web::post().to(register_client))
    );
}

