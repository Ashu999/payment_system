use actix_web::HttpRequest;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

#[derive(Serialize)]
pub struct AuthData {
    pub token: String,
}

pub fn create_token(user_id: Uuid) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: user_id,
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let jwt_secret_key = env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret_key.as_ref()),
    )
}

pub fn verify_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let jwt_secret_key = env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret_key.as_ref()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

pub fn extract_token(auth_header: Option<&actix_web::http::header::HeaderValue>) -> Option<String> {
    auth_header?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(String::from)
}

pub fn verify_request_token(req: &HttpRequest) -> Result<Claims, &'static str> {
    // Extract bearer token
    let auth_header = req.headers().get("Authorization");
    let token = extract_token(auth_header).ok_or("Invalid authorization header")?;

    // Verify JWT
    verify_token(&token).map_err(|_| "Invalid token")
}
