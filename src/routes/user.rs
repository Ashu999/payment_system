use crate::utils::token::{self};
use actix_web::{get, post, web, HttpResponse, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub balance: Decimal,
}

#[post("/user/register")]
pub async fn register(
    credentials: web::Json<UserCredentials>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Check if user exists
    let existing_user = sqlx::query!("SELECT id FROM users WHERE email = $1", credentials.email)
        .fetch_optional(&**pool)
        .await;

    if let Ok(Some(_)) = existing_user {
        return HttpResponse::Conflict().json("User already exists");
    }

    // Hash password
    let password_hash = hash(credentials.password.as_bytes(), DEFAULT_COST)
        .unwrap_or_else(|_| panic!("Failed to hash password"));

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
        Ok(_) => HttpResponse::Ok().json("User created successfully"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to create user"),
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
                let token = match token::create_token(user.id) {
                    Ok(token) => token,
                    Err(_) => {
                        return HttpResponse::InternalServerError().json("Failed to create token")
                    }
                };

                HttpResponse::Ok().json(AuthResponse { token })
            } else {
                HttpResponse::Unauthorized().json("Invalid credentials")
            }
        }
        _ => HttpResponse::Unauthorized().json("Invalid credentials"),
    }
}

#[get("/user")]
pub async fn get_user(
    req: actix_web::HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Extract and verify bearer token
    let auth_header = req.headers().get("Authorization");

    let token = match token::extract_token(auth_header) {
        Some(token) => token,
        None => return HttpResponse::Unauthorized().json("Invalid authorization header"),
    };

    // Verify JWT
    let claims = match token::verify_token(&token) {
        Ok(claims) => claims,
        Err(_) => return HttpResponse::Unauthorized().json("Invalid token"),
    };

    let user = sqlx::query!(
        "SELECT id, email, balance FROM users WHERE id = $1",
        claims.sub
    )
    .fetch_optional(&**pool)
    .await;

    match user {
        Ok(Some(user)) => HttpResponse::Ok().json(UserResponse {
            id: user.id,
            email: user.email,
            balance: user.balance,
        }),
        _ => HttpResponse::NotFound().json("User not found"),
    }
}
