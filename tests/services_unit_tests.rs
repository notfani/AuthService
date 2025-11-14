use MIREA_Backend_Courcer_paper::services::RegistrationError;

#[cfg(test)]
mod registration_error_tests {
    use super::*;

    #[test]
    fn test_username_exists_error_display() {
        let error = RegistrationError::UsernameExists;
        let error_message = format!("{}", error);

        assert!(error_message.contains("именем"));
        assert!(error_message.contains("существует"));
    }

    #[test]
    fn test_email_exists_error_display() {
        let error = RegistrationError::EmailExists;
        let error_message = format!("{}", error);

        assert!(error_message.contains("email"));
        assert!(error_message.contains("существует"));
    }

    #[test]
    fn test_hash_error_display() {
        let error = RegistrationError::HashError;
        let error_message = format!("{}", error);

        assert!(error_message.contains("хеширования"));
        assert!(error_message.contains("пароля"));
    }

    #[test]
    fn test_registration_errors_are_different() {
        let error1 = format!("{}", RegistrationError::UsernameExists);
        let error2 = format!("{}", RegistrationError::EmailExists);
        let error3 = format!("{}", RegistrationError::HashError);

        assert_ne!(error1, error2);
        assert_ne!(error2, error3);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_error_trait_implementation() {
        use std::error::Error;

        let error: Box<dyn Error> = Box::new(RegistrationError::UsernameExists);
        let display = format!("{}", error);

        assert!(!display.is_empty());
    }
}

#[cfg(test)]
mod bcrypt_password_hashing_tests {
    use bcrypt::{hash, verify, DEFAULT_COST};

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash_result = hash(password, DEFAULT_COST);

        assert!(hash_result.is_ok());
        let hashed = hash_result.unwrap();

        // Bcrypt хеш должен начинаться с $2b$ или $2a$
        assert!(hashed.starts_with("$2b$") || hashed.starts_with("$2a$"));
    }

    #[test]
    fn test_password_verification_success() {
        let password = "secure_password_123";
        let hashed = hash(password, DEFAULT_COST).unwrap();

        let verification = verify(password, &hashed);
        assert!(verification.is_ok());
        assert!(verification.unwrap());
    }

    #[test]
    fn test_password_verification_failure() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hashed = hash(password, DEFAULT_COST).unwrap();

        let verification = verify(wrong_password, &hashed);
        assert!(verification.is_ok());
        assert!(!verification.unwrap());
    }

    #[test]
    fn test_same_password_different_hashes() {
        let password = "same_password";
        let hash1 = hash(password, DEFAULT_COST).unwrap();
        let hash2 = hash(password, DEFAULT_COST).unwrap();

        // Два хеша одного пароля должны быть разными (соль)
        assert_ne!(hash1, hash2);

        // Но оба должны верифицироваться
        assert!(verify(password, &hash1).unwrap());
        assert!(verify(password, &hash2).unwrap());
    }

    #[test]
    fn test_hash_length() {
        let password = "test";
        let hashed = hash(password, DEFAULT_COST).unwrap();

        // Bcrypt хеш всегда фиксированной длины (60 символов)
        assert_eq!(hashed.len(), 60);
    }

    #[test]
    fn test_empty_password_hashing() {
        let password = "";
        let hash_result = hash(password, DEFAULT_COST);

        // Bcrypt должен хешировать даже пустой пароль
        assert!(hash_result.is_ok());
    }

    #[test]
    fn test_long_password_hashing() {
        let password = "a".repeat(1000);
        let hash_result = hash(password.as_str(), DEFAULT_COST);

        assert!(hash_result.is_ok());

        let hashed = hash_result.unwrap();
        assert!(verify(password.as_str(), &hashed).unwrap());
    }

    #[test]
    fn test_special_characters_in_password() {
        let password = "P@ssw0rd!#$%^&*()_+-=[]{}|;':\",./<>?";
        let hashed = hash(password, DEFAULT_COST).unwrap();

        assert!(verify(password, &hashed).unwrap());
    }

    #[test]
    fn test_unicode_password() {
        let password = "пароль密码🔒";
        let hashed = hash(password, DEFAULT_COST).unwrap();

        assert!(verify(password, &hashed).unwrap());
    }

    #[test]
    fn test_case_sensitive_verification() {
        let password = "Password123";
        let hashed = hash(password, DEFAULT_COST).unwrap();

        // Правильный пароль
        assert!(verify(password, &hashed).unwrap());

        // Неправильный регистр
        assert!(!verify("password123", &hashed).unwrap());
        assert!(!verify("PASSWORD123", &hashed).unwrap());
    }
}

#[cfg(test)]
mod uuid_generation_tests {
    use uuid::Uuid;

    #[test]
    fn test_uuid_generation() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        // Каждый UUID должен быть уникальным
        assert_ne!(uuid1, uuid2);
    }

    #[test]
    fn test_uuid_format() {
        let uuid = Uuid::new_v4();
        let uuid_string = uuid.to_string();

        // UUID должен быть в формате 8-4-4-4-12
        assert_eq!(uuid_string.len(), 36);
        assert_eq!(uuid_string.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn test_uuid_parsing() {
        let uuid = Uuid::new_v4();
        let uuid_string = uuid.to_string();

        let parsed = Uuid::parse_str(&uuid_string);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), uuid);
    }

    #[test]
    fn test_invalid_uuid_parsing() {
        let invalid_uuid = "not-a-valid-uuid";
        let parsed = Uuid::parse_str(invalid_uuid);

        assert!(parsed.is_err());
    }

    #[test]
    fn test_multiple_uuid_generation() {
        let mut uuids = vec![];
        for _ in 0..100 {
            uuids.push(Uuid::new_v4());
        }

        // Проверяем, что все UUID уникальны
        for i in 0..uuids.len() {
            for j in (i + 1)..uuids.len() {
                assert_ne!(uuids[i], uuids[j]);
            }
        }
    }
}

#[cfg(test)]
mod datetime_tests {
    use chrono::{Utc, Duration};

    #[test]
    fn test_current_datetime() {
        let now = Utc::now();
        let year = now.format("%Y").to_string().parse::<i32>().unwrap();

        assert!(year >= 2024);
    }

    #[test]
    fn test_datetime_comparison() {
        let time1 = Utc::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let time2 = Utc::now();

        assert!(time2 > time1);
    }

    #[test]
    fn test_datetime_formatting() {
        let now = Utc::now();
        let formatted = now.to_rfc3339();

        assert!(!formatted.is_empty());
        assert!(formatted.contains("T"));
    }

    #[test]
    fn test_datetime_duration() {
        let now = Utc::now();
        let future = now + Duration::hours(1);

        assert!(future > now);

        let diff = future - now;
        assert_eq!(diff.num_hours(), 1);
    }
}

#[cfg(test)]
mod string_operations_tests {
    #[test]
    fn test_string_trimming() {
        let username = "  test_user  ".to_string();
        let trimmed = username.trim();

        assert_eq!(trimmed, "test_user");
    }

    #[test]
    fn test_string_lowercase() {
        let email = "User@Example.COM".to_string();
        let lowercase = email.to_lowercase();

        assert_eq!(lowercase, "user@example.com");
    }

    #[test]
    fn test_string_contains() {
        let email = "user@example.com".to_string();

        assert!(email.contains("@"));
        assert!(email.contains("example"));
        assert!(!email.contains("test"));
    }

    #[test]
    fn test_string_splitting() {
        let email = "user@example.com".to_string();
        let parts: Vec<&str> = email.split('@').collect();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "user");
        assert_eq!(parts[1], "example.com");
    }

    #[test]
    fn test_string_length() {
        let username = "test_user";

        assert_eq!(username.len(), 9);
        assert!(username.len() >= 3);
        assert!(username.len() <= 50);
    }
}

#[cfg(test)]
mod json_serialization_tests {
    use serde_json;
    use MIREA_Backend_Courcer_paper::models::{RegisterUserRequest, ErrorResponse};

    #[test]
    fn test_register_request_to_json() {
        let request = RegisterUserRequest {
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_register_request_from_json() {
        let json = r#"{
            "username": "test_user",
            "email": "test@example.com",
            "password": "password123"
        }"#;

        let parsed: Result<RegisterUserRequest, _> = serde_json::from_str(json);
        assert!(parsed.is_ok());

        let request = parsed.unwrap();
        assert_eq!(request.username, "test_user");
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "password123");
    }

    #[test]
    fn test_error_response_serialization() {
        let error = ErrorResponse {
            error: "Test error".to_string(),
        };

        let json = serde_json::to_string(&error);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("Test error"));
    }

    #[test]
    fn test_invalid_json_parsing() {
        let invalid_json = r#"{ "username": "test" }"#; // Недостаточно полей

        let parsed: Result<RegisterUserRequest, _> = serde_json::from_str(invalid_json);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_json_with_extra_fields() {
        let json = r#"{
            "username": "test_user",
            "email": "test@example.com",
            "password": "password123",
            "extra_field": "ignored"
        }"#;

        let parsed: Result<RegisterUserRequest, _> = serde_json::from_str(json);
        assert!(parsed.is_ok());
    }
}

