use crate::utils::{
    auth::{self, AuthData},
    response::{json_response, ApiResponse, MessageData},
};
use actix_web::{get, post, web, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use email_address::EmailAddress;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserData {
    pub email: String,
    pub balance: Decimal,
}

#[post("/user/register")]
pub async fn register(
    credentials: web::Json<UserCredentials>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Validate email
    if !EmailAddress::is_valid(&credentials.email) {
        return json_response(ApiResponse::<MessageData>::error(
            400,
            "Invalid email".to_string(),
        ));
    }
    // Check if user exists
    let existing_user = sqlx::query!("SELECT id FROM users WHERE email = $1", credentials.email)
        .fetch_optional(&**pool)
        .await;

    if let Ok(Some(_)) = existing_user {
        return json_response(ApiResponse::<MessageData>::error(
            409,
            "User already exists".to_string(),
        ));
    }

    // Hash password
    let password_hash = match hash(credentials.password.as_bytes(), DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => {
            return json_response(ApiResponse::<MessageData>::error(
                500,
                "Internal server error".to_string(),
            ));
        }
    };

    let balance = Decimal::new(0, 0);

    // Create new user
    let user_id = Uuid::new_v4();
    let result = sqlx::query!(
        "INSERT INTO users (id, email, password_hash, balance) VALUES ($1, $2, $3, $4)",
        user_id,
        credentials.email,
        password_hash,
        balance
    )
    .execute(&**pool)
    .await;

    match result {
        Ok(_) => json_response(ApiResponse::success(MessageData {
            message: "User created successfully".to_string(),
        })),
        Err(_) => json_response(ApiResponse::<MessageData>::error(
            500,
            "Failed to create user".to_string(),
        )),
    }
}

#[post("/user/login")]
pub async fn login(
    credentials: web::Json<UserCredentials>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Fetch user
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE email = $1",
        credentials.email
    )
    .fetch_optional(&**pool)
    .await;

    match user {
        Ok(Some(user)) => {
            // Verify password
            if verify(&credentials.password, &user.password_hash).unwrap_or(false) {
                // Generate JWT
                match auth::create_token(user.id) {
                    Ok(token) => json_response(ApiResponse::success(AuthData { token })),
                    Err(_) => json_response(ApiResponse::<MessageData>::error(
                        500,
                        "Failed to create token".to_string(),
                    )),
                }
            } else {
                json_response(ApiResponse::<MessageData>::error(
                    401,
                    "Invalid credentials".to_string(),
                ))
            }
        }
        _ => json_response(ApiResponse::<MessageData>::error(
            401,
            "Invalid credentials".to_string(),
        )),
    }
}

#[get("/user")]
pub async fn get_user(
    req: actix_web::HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Verify token and get claims
    let claims = match auth::verify_request_token(&req) {
        Ok(claims) => claims,
        Err(msg) => return json_response(ApiResponse::<MessageData>::error(401, msg.to_string())),
    };

    let user = sqlx::query!(
        "SELECT id, email, balance FROM users WHERE id = $1",
        claims.sub
    )
    .fetch_optional(&**pool)
    .await;

    match user {
        Ok(Some(user)) => json_response(ApiResponse::success(UserData {
            email: user.email,
            balance: user.balance,
        })),
        _ => json_response(ApiResponse::<MessageData>::error(
            404,
            "User not found".to_string(),
        )),
    }
}
