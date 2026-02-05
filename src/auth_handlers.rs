use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use validator::Validate;
use bcrypt::verify;
use crate::models::{LoginRequest, ErrorResponse, RegisterUserResponse};
use crate::services::UserService;

// Login page (HTML form)
pub async fn login_page() -> impl Responder {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Вход</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 400px; margin: 50px auto; padding: 20px; }
        h1 { text-align: center; }
        form { display: flex; flex-direction: column; gap: 15px; }
        input { padding: 10px; border: 1px solid #ddd; border-radius: 4px; }
        button { padding: 10px; background-color: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; }
        button:hover { background-color: #0056b3; }
        .error { color: red; text-align: center; }
        .register-link { text-align: center; margin-top: 20px; }
    </style>
</head>
<body>
    <h1>Вход в систему</h1>
    <form action="/auth/login" method="POST" id="loginForm">
        <input type="email" name="email" placeholder="Email" required>
        <input type="password" name="password" placeholder="Пароль" required>
        <button type="submit">Войти</button>
    </form>
    <div class="register-link">
        <p>Нет аккаунта? <a href="/api/register">Зарегистрироваться</a></p>
    </div>
    <div class="error" id="error"></div>

    <script>
        const urlParams = new URLSearchParams(window.location.search);
        const error = urlParams.get('error');
        if (error) {
            document.getElementById('error').textContent = decodeURIComponent(error);
        }

        document.getElementById('loginForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const formData = new FormData(e.target);
            const data = {
                email: formData.get('email'),
                password: formData.get('password')
            };

            try {
                const response = await fetch('/auth/login', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });

                if (response.ok) {
                    const result = await response.json();
                    // Redirect to the original destination or default page
                    const returnTo = urlParams.get('return_to') || '/api/health';
                    window.location.href = returnTo;
                } else {
                    const error = await response.json();
                    document.getElementById('error').textContent = error.error || 'Ошибка входа';
                }
            } catch (err) {
                document.getElementById('error').textContent = 'Ошибка соединения';
            }
        });
    </script>
</body>
</html>
    "#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

// Login handler
pub async fn login(
    user_service: web::Data<UserService>,
    session: Session,
    request: web::Json<LoginRequest>,
) -> impl Responder {
    // Валидация
    if let Err(errors) = request.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("Validation error: {}", errors),
        });
    }

    // Получение пользователя по email
    match user_service.get_user_by_email(&request.email).await {
        Ok(Some(user)) => {
            // Проверка пароля
            match verify(&request.password, &user.password_hash) {
                Ok(true) => {
                    // Сохранение user_id в сессии
                    if let Err(e) = session.insert("user_id", user.id.to_string()) {
                        eprintln!("Session error: {}", e);
                        return HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Failed to create session".to_string(),
                        });
                    }

                    HttpResponse::Ok().json(RegisterUserResponse::from(user))
                }
                Ok(false) => HttpResponse::Unauthorized().json(ErrorResponse {
                    error: "Invalid credentials".to_string(),
                }),
                Err(e) => {
                    eprintln!("Password verification error: {}", e);
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    })
                }
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Invalid credentials".to_string(),
        }),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Internal server error".to_string(),
            })
        }
    }
}

// Logout handler
pub async fn logout(session: Session) -> impl Responder {
    session.purge();
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Logged out successfully"
    }))
}

// Get current user info
pub async fn me(
    user_service: web::Data<UserService>,
    session: Session,
) -> impl Responder {
    // Получение user_id из сессии
    match session.get::<String>("user_id") {
        Ok(Some(user_id_str)) => {
            // Парсинг UUID
            match user_id_str.parse::<uuid::Uuid>() {
                Ok(user_id) => {
                    // Получение пользователя
                    match user_service.get_user_by_id(user_id).await {
                        Ok(Some(user)) => HttpResponse::Ok().json(RegisterUserResponse::from(user)),
                        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
                            error: "User not found".to_string(),
                        }),
                        Err(e) => {
                            eprintln!("Database error: {}", e);
                            HttpResponse::InternalServerError().json(ErrorResponse {
                                error: "Internal server error".to_string(),
                            })
                        }
                    }
                }
                Err(_) => HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Invalid user ID in session".to_string(),
                }),
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Not authenticated".to_string(),
        }),
        Err(e) => {
            eprintln!("Session error: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Internal server error".to_string(),
            })
        }
    }
}

// Конфигурация маршрутов для аутентификации
pub fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::get().to(login_page))
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/me", web::get().to(me))
    );
}

