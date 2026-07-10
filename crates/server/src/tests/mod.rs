#[cfg(test)]
mod auth_tests {
    use base64::Engine;
    use uuid::Uuid;

    use crate::auth::verify_token;

    fn create_test_token(jwt_secret: &str, user_id: Uuid, role: &str) -> String {
        use chrono::{Duration, Utc};
        use jsonwebtoken::{encode, EncodingKey, Header};

        #[derive(serde::Serialize)]
        struct Claims {
            sub: String,
            exp: i64,
            role: String,
        }

        let exp = Utc::now() + Duration::days(7);
        let claims = Claims {
            sub: user_id.to_string(),
            exp: exp.timestamp(),
            role: role.to_string(),
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes()),
        )
        .expect("token creation")
    }

    #[test]
    fn test_verify_valid_token() {
        let secret = "test-secret-that-is-at-least-32-bytes-long";
        let user_id = Uuid::now_v7();
        let token = create_test_token(secret, user_id, "member");
        let result = verify_token(secret, &token);
        assert_eq!(result, Some(user_id));
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let secret = "test-secret-that-is-at-least-32-bytes-long";
        let wrong_secret = "wrong-secret--that-is-at-least-32-bytes";
        let user_id = Uuid::now_v7();
        let token = create_test_token(secret, user_id, "member");
        let result = verify_token(wrong_secret, &token);
        assert_eq!(result, None);
    }

    #[test]
    fn test_verify_expired_token() {
        use chrono::Utc;
        use jsonwebtoken::{encode, EncodingKey, Header};

        #[derive(serde::Serialize)]
        struct Claims {
            sub: String,
            exp: i64,
            role: String,
        }

        let secret = "test-secret-that-is-at-least-32-bytes-long";
        let exp = Utc::now().timestamp() - 3600;
        let claims = Claims {
            sub: Uuid::now_v7().to_string(),
            exp,
            role: "member".to_string(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("token creation");

        let result = verify_token(secret, &token);
        assert_eq!(result, None);
    }

    #[test]
    fn test_verify_garbage_token() {
        let secret = "test-secret-that-is-at-least-32-bytes-long";
        let result = verify_token(secret, "not.a.real.token.whatsoever");
        assert_eq!(result, None);
    }

    #[test]
    fn test_create_token_different_users_different_tokens() {
        let secret = "test-secret-that-is-at-least-32-bytes-long";
        let user_a = Uuid::now_v7();
        let user_b = Uuid::now_v7();
        let token_a = create_test_token(secret, user_a, "member");
        let token_b = create_test_token(secret, user_b, "member");
        assert_ne!(token_a, token_b);
        assert_eq!(verify_token(secret, &token_a), Some(user_a));
        assert_eq!(verify_token(secret, &token_b), Some(user_b));
    }

    #[test]
    fn test_create_token_role_included() {
        use jsonwebtoken::{decode, DecodingKey, Validation};

        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct Claims {
            sub: String,
            exp: i64,
            role: String,
        }

        let secret = "test-secret-that-is-at-least-32-bytes-long";
        let token = create_test_token(secret, Uuid::now_v7(), "superadmin");
        let data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .expect("decode");
        assert_eq!(data.claims.role, "superadmin");
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"hello komun test data 12345";
        let encoded = base64::engine::general_purpose::STANDARD.encode(original);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .expect("decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_reject_invalid() {
        let result = base64::engine::general_purpose::STANDARD.decode("!!!not-valid-base64!!!");
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod crypto_tests {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::RngCore;

    fn random_signing_key() -> SigningKey {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        SigningKey::from_bytes(&bytes)
    }

    #[test]
    fn test_ed25519_sign_verify_roundtrip() {
        let signing_key = random_signing_key();
        let verifying_key = signing_key.verifying_key();

        let message = b"komun-register:abc123challenge";
        let signature = signing_key.sign(message);
        verifying_key
            .verify_strict(message, &signature)
            .expect("verify must pass");
    }

    #[test]
    fn test_ed25519_wrong_message_fails() {
        let signing_key = random_signing_key();
        let verifying_key = signing_key.verifying_key();

        let signature = signing_key.sign(b"correct message");
        let result = verifying_key.verify_strict(b"wrong message", &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_ed25519_wrong_key_fails() {
        let signing_key_a = random_signing_key();
        let signing_key_b = random_signing_key();
        let verifying_key_b = signing_key_b.verifying_key();

        let message = b"test message";
        let signature = signing_key_a.sign(message);
        let result = verifying_key_b.verify_strict(message, &signature);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod config_tests {
    use crate::config::Config;

    #[test]
    fn test_config_defaults() {
        let toml = r#"
[server]
bind_address = "127.0.0.1"
port = 3000

[database]
url = "postgres://localhost/test"
max_connections = 5

[node]
name = "test-node"

[discovery]
directory_enabled = false

[auth]
jwt_secret = "this-is-at-least-32-characters-long-for-testing"
token_lifetime_days = 7

[federation]
enabled = false

[security]
allowed_origins = "*"

[posts]
max_kind_length = 64

[admin]
superadmin_public_keys = []
"#;
        let config: Config = toml::from_str(toml).expect("parse config");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.max_connections, 5);
        assert_eq!(config.relay.enabled, false);
        assert_eq!(config.relay.port, 9001);
        assert_eq!(config.relay.max_clients_per_room, 100);
    }

    #[test]
    fn test_config_missing_url_uses_default() {
        let toml = r#"
[server]
bind_address = "127.0.0.1"
port = 3000

[database]
max_connections = 5

[auth]
jwt_secret = "this-is-at-least-32-characters-long-for-testing"

[admin]
superadmin_public_keys = []
"#;
        let config: Config = toml::from_str(toml).expect("parse config");
        assert!(config.database.url.contains("localhost"));
        assert!(config.database.url.contains("postgres"));
        assert_eq!(config.database.max_connections, 5);
    }

    #[test]
    fn test_config_relay_disabled_by_default() {
        let toml = r#"
[server]
bind_address = "127.0.0.1"
port = 3000

[database]
url = "postgres://localhost/test"
max_connections = 5

[auth]
jwt_secret = "this-is-at-least-32-characters-long-for-testing"

[admin]
superadmin_public_keys = []
"#;
        let config: Config = toml::from_str(toml).expect("parse config");
        assert!(!config.relay.enabled);
        assert_eq!(config.relay.port, 9001);
    }
}
