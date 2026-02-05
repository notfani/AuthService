use MIREA_Backend_Courcer_paper::models::{
    RegisterUserRequest, RegisterUserResponse, User, ErrorResponse
};
use validator::Validate;
use uuid::Uuid;
use chrono::Utc;

#[cfg(test)]
mod model_validation_tests {
    use super::*;

    #[test]
    fn test_valid_register_request() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_username_too_short() {
        let request = RegisterUserRequest {
            username: "ab".to_string(), // Только 2 символа, нужно минимум 3
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        let result = request.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_username_too_long() {
        let request = RegisterUserRequest {
            username: "a".repeat(51), // 51 символ, максимум 50
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        let result = request.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_username_minimum_length() {
        let request = RegisterUserRequest {
            username: "abc".to_string(), // Ровно 3 символа
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_username_maximum_length() {
        let request = RegisterUserRequest {
            username: "a".repeat(50), // Ровно 50 символов
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_invalid_email_format() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "not_an_email".to_string(),
            password: "secure_password_123".to_string(),
        };

        let result = request.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_email_without_domain() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "user@".to_string(),
            password: "secure_password_123".to_string(),
        };

        let result = request.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_email_without_at_symbol() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "userexample.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        let result = request.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_email_formats() {
        let valid_emails = vec![
            "user@example.com",
            "user.name@example.com",
            "user+tag@example.co.uk",
            "user_123@test-domain.org",
        ];

        for email in valid_emails {
            let request = RegisterUserRequest {
                username: "valid_user".to_string(),
                email: email.to_string(),
                password: "secure_password_123".to_string(),
            };

            assert!(
                request.validate().is_ok(),
                "Email {} должен быть валидным",
                email
            );
        }
    }

    #[test]
    fn test_password_too_short() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "valid@example.com".to_string(),
            password: "short".to_string(), // Только 5 символов, нужно минимум 8
        };

        let result = request.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_password_minimum_length() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "valid@example.com".to_string(),
            password: "12345678".to_string(), // Ровно 8 символов
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_empty_username() {
        let request = RegisterUserRequest {
            username: "".to_string(),
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_empty_email() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_empty_password() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "valid@example.com".to_string(),
            password: "".to_string(),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_multiple_validation_errors() {
        let request = RegisterUserRequest {
            username: "ab".to_string(), // Слишком короткий
            email: "invalid".to_string(), // Невалидный email
            password: "123".to_string(), // Слишком короткий пароль
        };

        let result = request.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
        assert!(errors.field_errors().contains_key("email"));
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_unicode_in_username() {
        let request = RegisterUserRequest {
            username: "пользователь".to_string(),
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_special_characters_in_username() {
        let request = RegisterUserRequest {
            username: "user_name-123".to_string(),
            email: "valid@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_long_password() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "valid@example.com".to_string(),
            password: "a".repeat(100), // Очень длинный пароль
        };

        assert!(request.validate().is_ok());
    }
}

#[cfg(test)]
mod model_conversion_tests {
    use super::*;

    #[test]
    fn test_user_to_response_conversion() {
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();
        let updated_at = Utc::now();

        let user = User {
            id: user_id,
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "$2b$12$hashedpassword".to_string(),
            created_at,
            updated_at,
        };

        let response: RegisterUserResponse = user.into();

        assert_eq!(response.id, user_id);
        assert_eq!(response.username, "test_user");
        assert_eq!(response.email, "test@example.com");
        assert_eq!(response.created_at, created_at);
        // Проверяем, что пароль НЕ включен в response
    }

    #[test]
    fn test_user_struct_creation() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let user = User {
            id: user_id,
            username: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "$2b$12$somehash".to_string(),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "john_doe");
        assert_eq!(user.email, "john@example.com");
        assert!(user.password_hash.starts_with("$2b$"));
    }

    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse {
            error: "Тестовая ошибка".to_string(),
        };

        assert_eq!(error.error, "Тестовая ошибка");
    }

    #[test]
    fn test_register_request_serialization() {
        let request = RegisterUserRequest {
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&request);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("test_user"));
        assert!(json_str.contains("test@example.com"));
        assert!(json_str.contains("password123"));
    }

    #[test]
    fn test_register_response_serialization() {
        let response = RegisterUserResponse {
            id: Uuid::new_v4(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&response);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("test_user"));
        assert!(json_str.contains("test@example.com"));
    }
}

#[cfg(test)]
mod register_request_edge_cases {
    use super::*;

    #[test]
    fn test_whitespace_in_fields() {
        // Валидатор считает длину со всеми символами, включая пробелы
        let request = RegisterUserRequest {
            username: "  ab  ".to_string(), // 6 символов - валидация пройдёт
            email: "user@example.com".to_string(),
            password: "password123".to_string(),
        };

        // Валидация проходит, так как общая длина >= 3
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_username_only_whitespace() {
        // Только пробелы - валидация должна пройти по длине, но это бизнес-логика
        let request = RegisterUserRequest {
            username: "   ".to_string(), // 3 пробела
            email: "user@example.com".to_string(),
            password: "password123".to_string(),
        };

        // С точки зрения validator это валидно (длина = 3)
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_username_too_short_with_spaces() {
        let request = RegisterUserRequest {
            username: "  ".to_string(), // Только 2 символа (пробелы)
            email: "user@example.com".to_string(),
            password: "password123".to_string(),
        };

        // Это должно не пройти валидацию (длина < 3)
        assert!(request.validate().is_err());
    }


    #[test]
    fn test_email_with_subdomain() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "user@mail.example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_numeric_username() {
        let request = RegisterUserRequest {
            username: "12345".to_string(),
            email: "user@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_email_with_plus_sign() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "user+tag@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_password_with_special_characters() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "user@example.com".to_string(),
            password: "P@ssw0rd!#$%".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_password_with_spaces() {
        let request = RegisterUserRequest {
            username: "valid_user".to_string(),
            email: "user@example.com".to_string(),
            password: "pass word 123".to_string(),
        };

        assert!(request.validate().is_ok());
    }
}

