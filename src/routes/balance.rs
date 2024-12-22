use crate::utils::auth;
use actix_web::{get, post, web, HttpResponse, Responder};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AddBalanceRequest {
    amount: Decimal,
}

#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
    balance: Decimal,
}

#[get("/balance")]
pub async fn get_balance(
    req: actix_web::HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Verify token and get claims
    let claims = match auth::verify_request_token(&req) {
        Ok(claims) => claims,
        Err(msg) => return HttpResponse::Unauthorized().json(msg),
    };

    let user = sqlx::query!("SELECT balance FROM users WHERE id = $1", claims.sub)
        .fetch_optional(&**pool)
        .await;

    match user {
        Ok(Some(user)) => HttpResponse::Ok().json(BalanceResponse {
            balance: user.balance,
        }),
        _ => HttpResponse::NotFound().json("User not found"),
    }
}

#[post("/balance/add")]
pub async fn add_amount(
    req: actix_web::HttpRequest,
    add_request: web::Json<AddBalanceRequest>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Verify token and get claims
    let claims = match auth::verify_request_token(&req) {
        Ok(claims) => claims,
        Err(msg) => return HttpResponse::Unauthorized().json(msg),
    };

    // Validate amount is positive
    if add_request.amount <= Decimal::new(0, 0) {
        return HttpResponse::BadRequest().json("Amount must be positive");
    }

    // Update balance and create transaction record in a transaction
    let result = sqlx::query!(
        r#"
        WITH updated_user AS (
            UPDATE users 
            SET balance = balance + $1 
            WHERE id = $2 
            RETURNING balance
        )
        INSERT INTO transactions (id, user_id, transaction_type, amount, status)
        VALUES ($3, $2, 'RECEIVED', $1, 'SUCCESS')
        RETURNING (SELECT balance FROM updated_user)
        "#,
        add_request.amount,
        claims.sub,
        Uuid::new_v4(),
    )
    .fetch_one(&**pool)
    .await;

    match result {
        Ok(record) => HttpResponse::Ok().json(BalanceResponse {
            balance: record.balance.unwrap_or_default(),
        }),
        Err(_) => HttpResponse::InternalServerError().json("Failed to update balance"),
    }
}
