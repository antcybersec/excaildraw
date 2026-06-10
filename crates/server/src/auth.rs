use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_in: u64,
}

#[derive(Deserialize)]
pub struct TokenRequest {
    pub username: String,
}

pub fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "excaildraw-dev-secret-change-me".into()
        } else {
            panic!("JWT_SECRET environment variable must be set in release builds");
        }
    })
}

pub fn issue_token(username: &str) -> Option<TokenResponse> {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs()
        + 86400;
    let claims = Claims {
        sub: username.to_string(),
        exp: exp as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .ok()?;
    Some(TokenResponse {
        token,
        expires_in: 86400,
    })
}

pub fn validate_token(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|d| d.claims)
}

pub fn bearer_user(auth_header: Option<&str>) -> Option<String> {
    let header = auth_header?;
    let token = header.strip_prefix("Bearer ")?;
    validate_token(token).map(|c| c.sub)
}
